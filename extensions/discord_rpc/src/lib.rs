//! Discord Rich Presence for LumaIDE (native extension).
//!
//! Shows the most recently opened workspace, its project type and git branch.
//!
//! The extension ABI currently exposes no workspace/editor events to native
//! extensions, so a background thread polls `recent_workspaces.json` in the
//! Luma data directory and treats the entry with the newest `last_opened_at`
//! as the active workspace.
//!
//! Settings live in `<Luma data dir>/discord_rpc.json` (`%APPDATA%\Luma`). The file is
//! created on first run. Set `client_id` to your Discord application ID.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient, activity};
use luma_api::workspaces::RecentProjects;
use luma_api::{CoreAPI, ExtensionsCoreAPI, Shared};
use luma_extensions::abi::{LOG_INFO, LOG_WARN};
use luma_extensions::{LumaExtension, log};
use serde_json::Value;

const CONFIG_FILE: &str = "discord_rpc.json";
const RECENT_FILE: &str = "recent_workspaces.json";
const ACTIVE_FILE: &str = "active_file.json";
const POLL_CONNECTED: Duration = Duration::from_secs(3);

/// LumaIDE's Discord application ID. Application IDs are public, so it is
/// safe to ship. Users can override it with `client_id` in discord_rpc.json.
const DEFAULT_CLIENT_ID: &str = "1574685877295120394";

const DEFAULT_CONFIG: &str = r#"{
  "enabled": true,
  "show_workspace": true,
  "show_project_type": true,
  "show_git_branch": true
}
"#;

#[derive(Debug, Clone)]
struct Config {
    enabled: bool,
    client_id: String,
    show_workspace: bool,
    show_project_type: bool,
    show_git_branch: bool,
}

impl Config {
    fn load(dir: &Path) -> Self {
        let path = dir.join(CONFIG_FILE);
        if !path.exists() {
            if let Err(error) = std::fs::write(&path, DEFAULT_CONFIG) {
                log(LOG_WARN, &format!("Could not write {}: {error}", path.display()));
            }
        }
        let value = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .unwrap_or(Value::Null);
        let flag = |key: &str| value.get(key).and_then(Value::as_bool).unwrap_or(true);
        let text = |key: &str, default: &str| {
            value
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or(default)
                .trim()
                .to_string()
        };
        Self {
            enabled: flag("enabled"),
            client_id: Some(text("client_id", ""))
                .filter(|id| !id.is_empty())
                .unwrap_or_else(|| DEFAULT_CLIENT_ID.to_string()),
            show_workspace: flag("show_workspace"),
            show_project_type: flag("show_project_type"),
            show_git_branch: flag("show_git_branch"),
        }
    }
}

/// Languages with their own Rich Presence art asset (key must match the
/// asset name in the Discord developer portal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Language {
    Lua,
    Luau,
}

impl Language {
    fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "lua" => Some(Self::Lua),
            "luau" => Some(Self::Luau),
            _ => None,
        }
    }

    fn asset(self) -> &'static str {
        match self {
            Self::Lua => "lua",
            Self::Luau => "luau",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Lua => "Lua",
            Self::Luau => "Luau",
        }
    }
}

/// What is shown on the Discord profile. Compared to skip redundant updates.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Presence {
    details: String,
    state: Option<String>,
    /// Language of the focused file; `None` shows the Luma logo only.
    language: Option<Language>,
}

fn normalize(path: &Path) -> String {
    let text = path
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('/', "\\");
    if cfg!(windows) { text.to_lowercase() } else { text }
}

fn is_within(root: &Path, file: &Path) -> bool {
    let root = normalize(root);
    let root = root.trim_end_matches('\\');
    let file = normalize(file);
    file.strip_prefix(root)
        .is_some_and(|rest| rest.starts_with('\\'))
}

#[derive(Debug, Default)]
struct EditorState {
    in_editor: bool,
    workspace_path: Option<String>,
    workspace_title: Option<String>,
    path: Option<PathBuf>,
}

fn read_editor_state(data_dir: &Path) -> EditorState {
    let text = match std::fs::read_to_string(data_dir.join(ACTIVE_FILE)) {
        Ok(t) => t,
        Err(_) => return EditorState::default(),
    };
    let value: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return EditorState::default(),
    };
    let in_editor = value
        .get("in_editor")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let workspace_path = value
        .get("workspace_path")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string());
    let workspace_title = value
        .get("workspace_title")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string());
    let path = value
        .get("path")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from);

    EditorState {
        in_editor,
        workspace_path,
        workspace_title,
        path,
    }
}

fn project_type_label(id: &str) -> String {
    match id {
        "lua" => "Lua".into(),
        "luajit" => "LuaJIT".into(),
        "luau" => "Luau".into(),
        "roblox" => "Roblox".into(),
        "computercraft" => "ComputerCraft".into(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        }
    }
}

fn read_project_type(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join(".luma").join("project.json")).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    let id = value.get("project_type")?.as_str()?.trim();
    (!id.is_empty()).then(|| project_type_label(id))
}

fn read_git_branch(root: &Path) -> Option<String> {
    let head = std::fs::read_to_string(root.join(".git").join("HEAD")).ok()?;
    let head = head.trim();
    if let Some(branch) = head.strip_prefix("ref: refs/heads/") {
        return Some(branch.to_string());
    }
    (head.len() >= 7).then(|| head[..7].to_string())
}

fn latest_workspace(data_dir: &Path) -> Option<(String, PathBuf)> {
    let text = std::fs::read_to_string(data_dir.join(RECENT_FILE)).ok()?;
    let recent: RecentProjects = serde_json::from_str(&text).ok()?;
    let face = recent
        .map
        .values()
        .filter(|face| Path::new(&face.path).is_dir())
        .max_by_key(|face| face.last_opened_at.unwrap_or(0))?;
    let root = PathBuf::from(&face.path);
    let title = if face.title.trim().is_empty() {
        root.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "workspace".into())
    } else {
        face.title.clone()
    };
    Some((title, root))
}

fn build_presence(config: &Config, data_dir: &Path) -> Presence {
    let editor_state = read_editor_state(data_dir);

    // If the user is in the project selection screen and not in the actual
    // IDE editor, simply say "Idle" with the Luma image.
    if !editor_state.in_editor {
        return Presence {
            details: "Idle".into(),
            state: None,
            language: None,
        };
    }

    // When in editor, determine workspace title and root directory from the editor state
    // or fall back to the most recent workspace.
    let (title, root) = match (editor_state.workspace_title, editor_state.workspace_path) {
        (Some(title), Some(path)) if !path.trim().is_empty() => (title, PathBuf::from(path)),
        _ => match latest_workspace(data_dir) {
            Some((title, root)) => (title, root),
            None => {
                return Presence {
                    details: "Idle".into(),
                    state: None,
                    language: None,
                };
            }
        },
    };

    let details = if config.show_workspace {
        format!("Working on {title}")
    } else {
        "Working in LumaIDE".into()
    };

    let file = editor_state
        .path
        .filter(|file| is_within(&root, file));
    let language = file.as_deref().and_then(Language::from_path);

    let mut parts = Vec::new();
    if let Some(file) = &file {
        if let Some(name) = file.file_name() {
            parts.push(format!("Editing {}", name.to_string_lossy()));
        }
    } else if config.show_project_type {
        parts.extend(read_project_type(&root));
    }

    if config.show_git_branch {
        parts.extend(read_git_branch(&root).map(|branch| format!("branch: {branch}")));
    }

    Presence {
        details,
        state: (!parts.is_empty()).then(|| parts.join(" | ")),
        language,
    }
}

fn to_activity(presence: &Presence, started: i64) -> activity::Activity<'_> {
    // Focused Lua/Luau file: language icon large, Luma logo small.
    // Otherwise the Luma logo is the large image.
    let assets = match presence.language {
        Some(language) => activity::Assets::new()
            .large_image(language.asset())
            .large_text(language.label())
            .small_image("luma")
            .small_text("LumaIDE"),
        None => activity::Assets::new()
            .large_image("luma")
            .large_text("LumaIDE"),
    };
    let mut built = activity::Activity::new()
        .details(presence.details.as_str())
        .timestamps(activity::Timestamps::new().start(started))
        .assets(assets);
    if let Some(state) = &presence.state {
        built = built.state(state.as_str());
    }
    built
}

fn run(data_dir: PathBuf, started: i64, stop: Receiver<()>) {
    let mut client: Option<DiscordIpcClient> = None;
    let mut last: Option<Presence> = None;
    let mut warned_offline = false;

    loop {
        // Re-read the config every cycle so the `enabled` toggle (and the
        // display flags) take effect without restarting Luma.
        let config = Config::load(&data_dir);

        if !config.enabled {
            // RPC has been turned off: clear any activity and idle until it
            // is turned back on.
            if let Some(mut active) = client.take() {
                let _ = active.clear_activity();
                let _ = active.close();
            }
            last = None;
            warned_offline = false;
        } else if client.is_none() {
            let mut candidate = DiscordIpcClient::new(&config.client_id);
            match candidate.connect() {
                Ok(()) => {
                    log(LOG_INFO, "Discord RPC connected");
                    client = Some(candidate);
                    last = None;
                    warned_offline = false;
                }
                Err(error) => {
                    if !warned_offline {
                        log(
                            LOG_INFO,
                            &format!("Discord not reachable ({error}); retrying in the background"),
                        );
                        warned_offline = true;
                    }
                }
            }
        }

        if let Some(active) = client.as_mut() {
            let presence = build_presence(&config, &data_dir);
            if last.as_ref() != Some(&presence) {
                let result = active.set_activity(to_activity(&presence, started));
                match result {
                    Ok(()) => last = Some(presence),
                    Err(error) => {
                        log(LOG_WARN, &format!("Discord RPC update failed: {error}"));
                        client = None;
                    }
                }
            }
        }

        match stop.recv_timeout(POLL_CONNECTED) {
            Err(RecvTimeoutError::Timeout) => continue,
            _ => break,
        }
    }

    if let Some(mut active) = client {
        let _ = active.clear_activity();
        let _ = active.close();
    }
}

#[derive(Default)]
struct DiscordRpcExtension {
    stop: Option<Sender<()>>,
    worker: Option<JoinHandle<()>>,
}

impl LumaExtension for DiscordRpcExtension {
    fn name(&self) -> &str {
        "discord-rpc"
    }

    fn initialize(
        &mut self,
        core_api: &Shared<dyn CoreAPI>,
        _extensions_core: &Shared<dyn ExtensionsCoreAPI>,
    ) -> Result<()> {
        // Note: get_configuration_dir() is the path of configuration.toml (a
        // file), not a directory. Never create or write inside it.
        let data_dir = core_api.get_data_dir().clone();

        let config = Config::load(&data_dir);
        if config.client_id.is_empty() {
            log(
                LOG_WARN,
                &format!(
                    "Discord RPC idle: no application ID. Set DEFAULT_CLIENT_ID or \"client_id\" in {}",
                    data_dir.join(CONFIG_FILE).display()
                ),
            );
            return Ok(());
        }

        // Always spawn the worker so the `enabled` toggle (and display flags)
        // in discord_rpc.json take effect while Luma is running. The worker
        // re-reads the config on every poll cycle and idles (clearing any
        // activity) when disabled.
        let started = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs() as i64)
            .unwrap_or(0);
        let (sender, receiver) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("discord-rpc".into())
            .spawn(move || run(data_dir, started, receiver))
            .context("Failed to start the Discord RPC thread")?;
        self.stop = Some(sender);
        self.worker = Some(worker);
        log(LOG_INFO, "Discord RPC started");
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        // Wake the worker; it clears the activity and exits. Joining matters:
        // the library must not unload while the thread is still running.
        if let Some(sender) = self.stop.take() {
            let _ = sender.send(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        Ok(())
    }
}

luma_extensions::declare_luma_extension!(DiscordRpcExtension);
