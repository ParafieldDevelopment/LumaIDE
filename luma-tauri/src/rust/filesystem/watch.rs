//! Per-IDE-window filesystem watching.
//!
//! The watcher runs for the active workspace of each IDE window. Events are
//! debounced, filtered (VCS/metadata/temp files are ignored), self-write
//! suppressed, and only emitted to a window while it is focused.

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

const DEBOUNCE_MS: u64 = 200;
const SELF_WRITE_WINDOW: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    pub kind: &'static str,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilesChangedPayload {
    pub changes: Vec<FileChange>,
}

struct WatchState {
    // Dropping the watcher stops notifications for this window.
    _watcher: RecommendedWatcher,
}

static WATCHERS: OnceLock<RwLock<HashMap<String, WatchState>>> = OnceLock::new();
static SELF_WRITES: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();

fn watchers() -> &'static RwLock<HashMap<String, WatchState>> {
    WATCHERS.get_or_init(|| RwLock::new(HashMap::new()))
}

fn self_writes() -> &'static Mutex<HashMap<String, Instant>> {
    SELF_WRITES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Stable, case-insensitive key for comparing paths from different sources.
pub fn path_key(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let text = text.strip_prefix("//?/").unwrap_or(&text);
    text.to_lowercase()
}

/// Remember that Luma itself is about to touch this path so the watcher does
/// not report it back as an external change.
pub fn suppress_self_write(path: &Path) {
    if let Ok(mut writes) = self_writes().lock() {
        writes.insert(path_key(path), Instant::now());
    }
}

fn is_self_write(path: &Path) -> bool {
    let Ok(mut writes) = self_writes().lock() else {
        return false;
    };
    let now = Instant::now();
    writes.retain(|_, time| now.duration_since(*time) < SELF_WRITE_WINDOW);
    writes.contains_key(&path_key(path))
}

fn is_watched(path: &Path, root: &Path) -> bool {
    if !path_key(path).starts_with(&path_key(root)) {
        return false;
    }
    !path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        name == ".git" || name == ".luma" || name.ends_with(".tmp")
    })
}

fn change_for(event: &Event, path: &Path, root: &Path) -> Option<FileChange> {
    if !is_watched(path, root) || is_self_write(path) {
        return None;
    }
    let kind = match &event.kind {
        EventKind::Create(_) => "create",
        EventKind::Modify(_) => "modify",
        EventKind::Remove(_) => "remove",
        _ => "other",
    };
    let is_dir = matches!(
        &event.kind,
        EventKind::Create(notify::event::CreateKind::Folder)
            | EventKind::Remove(notify::event::RemoveKind::Folder)
    ) || path.is_dir();
    Some(FileChange {
        path: path.to_string_lossy().into_owned(),
        kind,
        is_dir,
    })
}

/// Start (or replace) the watcher for an IDE window.
pub fn start(app: AppHandle, label: String, root: PathBuf) {
    stop(&label);

    let buffer = Arc::new(Mutex::new(Vec::<FileChange>::new()));
    let generation = Arc::new(AtomicU64::new(0));
    let app_for_events = app.clone();
    let label_for_events = label.clone();
    let buffer_for_events = Arc::clone(&buffer);
    let generation_for_events = Arc::clone(&generation);
    let root_for_filter = root.clone();

    let watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
        let Ok(event) = result else {
            return;
        };
        let changes: Vec<FileChange> = event
            .paths
            .iter()
            .filter_map(|path| change_for(&event, path, &root_for_filter))
            .collect();
        if changes.is_empty() {
            return;
        }

        if let Ok(mut pending) = buffer_for_events.lock() {
            pending.extend(changes);
        }
        let current = generation_for_events.fetch_add(1, Ordering::SeqCst) + 1;

        let app = app_for_events.clone();
        let label = label_for_events.clone();
        let buffer = Arc::clone(&buffer_for_events);
        let generation = Arc::clone(&generation_for_events);
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_millis(DEBOUNCE_MS)).await;
            // A newer event reset the timer; that task will flush the batch.
            if generation.load(Ordering::SeqCst) != current {
                return;
            }
            // Only refresh the file tree of the window the user is looking at.
            let focused = app
                .get_webview_window(&label)
                .and_then(|window| window.is_focused().ok())
                .unwrap_or(false);
            if !focused {
                let _ = buffer.lock().map(|mut pending| pending.clear());
                return;
            }
            let changes = buffer
                .lock()
                .map(|mut pending| std::mem::take(&mut *pending))
                .unwrap_or_default();
            if changes.is_empty() {
                return;
            }
            if let Err(error) = app.emit_to(
                &label,
                "workspace-files-changed",
                FilesChangedPayload { changes },
            ) {
                log::warn!("Failed to emit file changes to {label}: {error}");
            }
        });
    });

    match watcher {
        Ok(mut watcher) => {
            if let Err(error) = watcher.watch(&root, RecursiveMode::Recursive) {
                log::warn!("Failed to watch {}: {error}", root.display());
                return;
            }
            match watchers().write() {
                Ok(mut map) => {
                    map.insert(label.clone(), WatchState { _watcher: watcher });
                }
                Err(error) => log::warn!("Failed to store watcher for {label}: {error}"),
            }
        }
        Err(error) => log::warn!("Failed to create filesystem watcher for {label}: {error}"),
    }
}

/// Stop watching a window (or when a different workspace is opened in it).
pub fn stop(label: &str) {
    match watchers().write() {
        Ok(mut map) => {
            map.remove(label);
        }
        Err(error) => log::warn!("Failed to stop watcher for {label}: {error}"),
    }
}
