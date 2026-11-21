use crate::settings_window::settings_window;
use anyhow::Context;
use luma_api::projects::{Project, RecentProjects};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{OnceLock, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use uuid::Uuid;

/// Workspace open in each IDE window, keyed by window label ("main", "main-2", ...).
static WINDOW_WORKSPACES: OnceLock<RwLock<HashMap<String, Project>>> = OnceLock::new();

fn window_workspaces() -> &'static RwLock<HashMap<String, Project>> {
    WINDOW_WORKSPACES.get_or_init(|| RwLock::new(HashMap::new()))
}

/// The workspace currently open in the IDE window with this label.
pub fn workspace_for_window(label: &str) -> Option<Project> {
    window_workspaces().read().ok()?.get(label).cloned()
}

fn is_ide_label(label: &str) -> bool {
    label == "main" || label.starts_with("main-")
}

pub(crate) fn free_main_label(app: &tauri::AppHandle) -> String {
    if app.get_webview_window("main").is_none() {
        return "main".to_string();
    }
    let mut n = 2;
    loop {
        let label = format!("main-{n}");
        if app.get_webview_window(&label).is_none() {
            return label;
        }
        n += 1;
    }
}

/// "This window" reuses the calling IDE window; otherwise (new window, or called
/// from the startup window) the lowest free IDE label is used.
fn target_window_label(
    app: &tauri::AppHandle,
    caller: &tauri::WebviewWindow,
    new_window: bool,
) -> String {
    if !new_window && is_ide_label(caller.label()) {
        return caller.label().to_string();
    }
    free_main_label(app)
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Strip the Windows verbatim prefix (`\\?\C:\...`) that `canonicalize` adds so
/// stored and displayed paths look normal. `\\?\UNC\...` paths are left alone.
pub(crate) fn clean_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let text = path.to_string_lossy();
        if let Some(rest) = text.strip_prefix(r"\\?\") {
            let bytes = rest.as_bytes();
            if bytes.len() >= 3
                && bytes[0].is_ascii_alphabetic()
                && bytes[1] == b':'
                && bytes[2] == b'\\'
            {
                return PathBuf::from(rest);
            }
        }
    }
    path
}

fn clean_path_string(path: &str) -> String {
    clean_path(PathBuf::from(path))
        .to_string_lossy()
        .into_owned()
}

fn detect_git_branch(workspace_path: &Path) -> Option<String> {
    let head_path = workspace_path.join(".git").join("HEAD");
    if let Ok(content) = std::fs::read_to_string(head_path) {
        let content = content.trim();
        if let Some(branch) = content.strip_prefix("ref: refs/heads/") {
            return Some(branch.to_string());
        }
        if content.len() >= 7 {
            return Some(content[..7].to_string());
        }
    }
    None
}

#[derive(Debug, Serialize, Deserialize)]
struct WorkspaceMetadata {
    project_uuid: String,
    title: String,
    #[serde(default)]
    created_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    project_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    runtime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    template: Option<String>,
}

/// Validate an optional wizard id (project type, runtime or template).
/// Backend only checks the format. The set of valid ids is owned by the
/// frontend registry and extensions.
fn validate_choice_id(
    label: &str,
    value: Option<String>,
) -> Result<Option<String>, tauri::ipc::InvokeError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() > 64
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err(tauri::ipc::InvokeError::from_anyhow(anyhow::anyhow!(
            "Invalid {label}"
        )));
    }
    Ok(Some(value.to_string()))
}

fn write_json_file(path: &Path, contents: &str) -> Result<(), tauri::ipc::InvokeError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .context("Failed to create metadata directory")
            .map_err(tauri::ipc::InvokeError::from_anyhow)?;
    }

    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, contents)
        .context(format!(
            "Failed to write temporary file {}",
            temporary.display()
        ))
        .map_err(tauri::ipc::InvokeError::from_anyhow)?;
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(tauri::ipc::InvokeError::from_anyhow(
            anyhow::Error::from(error).context(format!("Failed to replace {}", path.display())),
        ));
    }
    Ok(())
}

fn load_or_initialize_workspace_metadata(
    workspace_path: &Path,
    default_title: &str,
) -> Result<WorkspaceMetadata, tauri::ipc::InvokeError> {
    let metadata_path = workspace_path.join(".luma").join("project.json");
    if metadata_path.exists() {
        let contents = std::fs::read_to_string(&metadata_path)
            .context(format!("Failed to read {}", metadata_path.display()))
            .map_err(tauri::ipc::InvokeError::from_anyhow)?;
        let metadata: WorkspaceMetadata = serde_json::from_str(&contents)
            .context(format!(
                "Invalid workspace metadata at {}",
                metadata_path.display()
            ))
            .map_err(tauri::ipc::InvokeError::from_anyhow)?;
        Uuid::parse_str(&metadata.project_uuid)
            .context("Workspace metadata contains an invalid UUID")
            .map_err(tauri::ipc::InvokeError::from_anyhow)?;
        if metadata.title.trim().is_empty() {
            return Err(tauri::ipc::InvokeError::from_anyhow(anyhow::anyhow!(
                "Workspace metadata has an empty title"
            )));
        }
        return Ok(metadata);
    }

    let recent_path = recent_workspaces_path()?;
    let recent = load_recent_workspaces(&recent_path)?;
    let workspace_path_string = workspace_path.to_string_lossy();
    let existing_uuid = recent
        .map
        .iter()
        .find(|(_, project)| same_path(&project.path, workspace_path_string.as_ref()))
        .map(|(id, project)| {
            if project.project_uuid.trim().is_empty() {
                id.clone()
            } else {
                project.project_uuid.clone()
            }
        });

    let metadata = WorkspaceMetadata {
        project_uuid: existing_uuid.unwrap_or_else(|| Uuid::new_v4().to_string()),
        title: default_title.to_string(),
        created_at: unix_timestamp(),
        project_type: None,
        runtime: None,
        template: None,
    };
    let contents = serde_json::to_string_pretty(&metadata)
        .context("Failed to serialize workspace metadata")
        .map_err(tauri::ipc::InvokeError::from_anyhow)?;
    write_json_file(&metadata_path, &contents)?;
    Ok(metadata)
}

fn recent_workspaces_path() -> Result<PathBuf, tauri::ipc::InvokeError> {
    let core = luma_core::CORE_API.get().ok_or_else(|| {
        tauri::ipc::InvokeError::from_anyhow(anyhow::anyhow!("Luma core is not initialized"))
    })?;
    Ok(core.get_data_dir().join("recent_workspaces.json"))
}

fn load_recent_workspaces(path: &Path) -> Result<RecentProjects, tauri::ipc::InvokeError> {
    match std::fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents)
            .context(format!(
                "Failed to parse recent workspaces from {}",
                path.display()
            ))
            .map_err(tauri::ipc::InvokeError::from_anyhow),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(RecentProjects::default()),
        Err(error) => Err(tauri::ipc::InvokeError::from_anyhow(
            anyhow::Error::from(error).context(format!(
                "Failed to read recent workspaces from {}",
                path.display()
            )),
        )),
    }
}

fn save_recent_workspaces(
    path: &Path,
    recent: &RecentProjects,
) -> Result<(), tauri::ipc::InvokeError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .context("Failed to create Luma data directory")
            .map_err(tauri::ipc::InvokeError::from_anyhow)?;
    }

    let contents = serde_json::to_string_pretty(recent)
        .context("Failed to serialize recent workspaces")
        .map_err(tauri::ipc::InvokeError::from_anyhow)?;
    std::fs::write(path, contents)
        .context(format!(
            "Failed to save recent workspaces to {}",
            path.display()
        ))
        .map_err(tauri::ipc::InvokeError::from_anyhow)
}

fn same_path(left: &str, right: &str) -> bool {
    let left = Path::new(left)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(left));
    let right = Path::new(right)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(right));
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

fn record_recent_workspace(mut project: Project) -> Result<Project, tauri::ipc::InvokeError> {
    let recent_path = recent_workspaces_path()?;
    let mut recent = load_recent_workspaces(&recent_path)?;

    if let Some((existing_id, existing)) = recent
        .map
        .iter()
        .find(|(_, entry)| same_path(&entry.path, &project.path))
    {
        if project.project_uuid.trim().is_empty() {
            project.project_uuid = existing.project_uuid.clone();
            if project.project_uuid.trim().is_empty() {
                project.project_uuid = existing_id.clone();
            }
        }
    }

    if project.project_uuid.trim().is_empty() {
        project.project_uuid = Uuid::new_v4().to_string();
    }
    project.last_opened_at = Some(unix_timestamp());

    recent
        .map
        .retain(|_, entry| !same_path(&entry.path, &project.path));
    recent
        .map
        .insert(project.project_uuid.clone(), project.clone());
    save_recent_workspaces(&recent_path, &recent)?;

    Ok(project)
}

fn open_main_window(app: &tauri::AppHandle, project: &Project, label: &str) -> anyhow::Result<()> {
    let title = format!("Luma â€” {}", project.title);

    let window = if let Some(window) = app.get_webview_window(label) {
        window.set_title(&title)?;
        window
    } else {
        let window = WebviewWindowBuilder::new(app, label, WebviewUrl::App("windows/main.html".into()))
            .visible(false)
            .decorations(!cfg!(target_os = "windows"))
            .min_inner_size(1050.0, 600.0)
            .title(&title)
            .background_color(crate::STARTUP_COLOR)
            .build()
            .context("Failed to create the IDE window")?;

        // Forget this window's workspace once it is gone.
        let owned_label = label.to_string();
        window.on_window_event(move |event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                if let Ok(mut map) = window_workspaces().write() {
                    map.remove(&owned_label);
                }
            }
        });
        window
    };

    window.show().context("Failed to show the IDE window")?;
    window
        .set_focus()
        .context("Failed to focus the IDE window")?;

    // No global broadcast: each IDE window loads its own workspace via
    // `get_active_workspace` (new windows) or the `open_workspace` return value.
    if let Some(startup) = app.get_webview_window("startup") {
        if startup.label() != window.label() {
            startup
                .close()
                .context("Failed to close workspace selection window")?;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn get_recent_workspaces() -> Result<Vec<Project>, tauri::ipc::InvokeError> {
    let recent_path = recent_workspaces_path()?;
    let mut recent = load_recent_workspaces(&recent_path)?;
    let mut missing = Vec::new();

    let mut workspaces: Vec<Project> = recent
        .map
        .iter()
        .filter_map(|(project_uuid, project)| {
            if !Path::new(&project.path).is_dir() {
                missing.push(project_uuid.clone());
                return None;
            }

            let mut project = project.clone();
            // Older entries were saved with a `\\?\` prefix; clean them on read.
            project.path = clean_path_string(&project.path);
            project.git_branch = detect_git_branch(Path::new(&project.path));
            if project.project_uuid.trim().is_empty() {
                project.project_uuid = project_uuid.clone();
            }
            project.icon_path = project.icon_path.map(|icon_path| {
                let icon_path = PathBuf::from(icon_path);
                if icon_path.is_absolute() {
                    icon_path
                } else {
                    luma_core::data_dir().unwrap_or_default().join(icon_path)
                }
                .to_string_lossy()
                .into_owned()
            });
            Some(project)
        })
        .collect();

    if !missing.is_empty() {
        for project_uuid in missing {
            recent.map.remove(&project_uuid);
        }
        save_recent_workspaces(&recent_path, &recent)?;
    }

    workspaces.sort_by(|left, right| {
        right
            .last_opened_at
            .unwrap_or_default()
            .cmp(&left.last_opened_at.unwrap_or_default())
    });
    Ok(workspaces)
}

#[tauri::command]
pub async fn load_tool_window_layout(workspace_path: String) -> Result<Option<String>, tauri::ipc::InvokeError> {
    let workspace = PathBuf::from(&workspace_path).canonicalize().context("Workspace folder does not exist").map_err(tauri::ipc::InvokeError::from_anyhow)?;
    if !workspace.is_dir() { return Err(tauri::ipc::InvokeError::from_anyhow(anyhow::anyhow!("Workspace path is not a directory"))); }
    let path = workspace.join(".luma").join("tool-windows.json");
    match std::fs::read_to_string(path) {
        Ok(contents) => { serde_json::from_str::<serde_json::Value>(&contents).context("Invalid tool-window layout JSON").map_err(tauri::ipc::InvokeError::from_anyhow)?; Ok(Some(contents)) },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(tauri::ipc::InvokeError::from_anyhow(anyhow::Error::from(error).context("Failed to read tool-window layout"))),
    }
}

#[tauri::command]
pub async fn save_tool_window_layout(workspace_path: String, layout: String) -> Result<(), tauri::ipc::InvokeError> {
    let workspace = PathBuf::from(&workspace_path).canonicalize().context("Workspace folder does not exist").map_err(tauri::ipc::InvokeError::from_anyhow)?;
    if !workspace.is_dir() { return Err(tauri::ipc::InvokeError::from_anyhow(anyhow::anyhow!("Workspace path is not a directory"))); }
    let value: serde_json::Value = serde_json::from_str(&layout).context("Invalid tool-window layout JSON").map_err(tauri::ipc::InvokeError::from_anyhow)?;
    if value.get("version").and_then(|v| v.as_u64()).is_none() { return Err(tauri::ipc::InvokeError::from_anyhow(anyhow::anyhow!("Unsupported tool-window layout version"))); }
    let path = workspace.join(".luma").join("tool-windows.json");
    let serialized = serde_json::to_string_pretty(&value)
        .map_err(|error| tauri::ipc::InvokeError::from_anyhow(anyhow::Error::from(error)))?;
    write_json_file(&path, &serialized)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateFile {
    pub path: String,
    pub content: String,
}

fn validate_template_file_path(path_str: &str) -> anyhow::Result<PathBuf> {
    let path = Path::new(path_str);
    if path.is_absolute() {
        anyhow::bail!("Template file path must be relative: {}", path_str);
    }
    for component in path.components() {
        match component {
            std::path::Component::Normal(c) => {
                let name = c.to_string_lossy();
                if name.contains('\0') {
                    anyhow::bail!("Template filename cannot contain null bytes: {}", path_str);
                }
                #[cfg(windows)]
                {
                    if name.chars().any(|ch| "<>:\"|?*".contains(ch)) {
                        anyhow::bail!("Template filename contains invalid characters: {}", path_str);
                    }
                }
            }
            std::path::Component::CurDir => continue,
            _ => anyhow::bail!("Invalid path traversal in template file: {}", path_str),
        }
    }
    Ok(path.to_path_buf())
}

#[tauri::command]
pub async fn create_workspace(
    title: String,
    parent_path: String,
    project_type: Option<String>,
    runtime: Option<String>,
    template: Option<String>,
    files: Option<Vec<TemplateFile>>,
) -> Result<Project, tauri::ipc::InvokeError> {
    let project_type = validate_choice_id("project type", project_type)?;
    let runtime = validate_choice_id("runtime", runtime)?;
    let template = validate_choice_id("template", template)?;

    let title = title.trim();
    if title.is_empty() {
        return Err(tauri::ipc::InvokeError::from_anyhow(anyhow::anyhow!(
            "Workspace name cannot be empty"
        )));
    }
    if title
        .chars()
        .any(|character| character.is_control() || "/\\<>:\"|?*".contains(character))
        || title.ends_with('.')
        || title.ends_with(' ')
    {
        return Err(tauri::ipc::InvokeError::from_anyhow(anyhow::anyhow!(
            "Workspace name contains invalid characters"
        )));
    }

    let parent = PathBuf::from(parent_path);
    let parent = parent
        .canonicalize()
        .context("Selected parent folder does not exist")
        .map_err(tauri::ipc::InvokeError::from_anyhow)?;
    let parent = clean_path(parent);
    if !parent.is_dir() {
        return Err(tauri::ipc::InvokeError::from_anyhow(anyhow::anyhow!(
            "Selected location is not a directory"
        )));
    }

    let workspace_path = parent.join(title);
    std::fs::create_dir(&workspace_path)
        .context(format!(
            "Could not create workspace folder {}",
            workspace_path.display()
        ))
        .map_err(tauri::ipc::InvokeError::from_anyhow)?;

    let config_dir = workspace_path.join(".luma");
    if let Err(error) = std::fs::create_dir(&config_dir).and_then(|_| {
        std::fs::write(
            config_dir.join("workspace.toml"),
            "# Luma workspace configuration\n",
        )
    }) {
        let _ = std::fs::remove_dir_all(&workspace_path);
        return Err(tauri::ipc::InvokeError::from_anyhow(
            anyhow::Error::from(error).context("Failed to initialize workspace configuration"),
        ));
    }

    let metadata = WorkspaceMetadata {
        project_uuid: Uuid::new_v4().to_string(),
        title: title.to_string(),
        created_at: unix_timestamp(),
        project_type: project_type.clone(),
        runtime: runtime.clone(),
        template: template.clone(),
    };
    let metadata_contents = serde_json::to_string_pretty(&metadata)
        .context("Failed to serialize workspace metadata")
        .map_err(tauri::ipc::InvokeError::from_anyhow)?;
    if let Err(error) = write_json_file(
        &workspace_path.join(".luma").join("project.json"),
        &metadata_contents,
    ) {
        let _ = std::fs::remove_dir_all(&workspace_path);
        return Err(error);
    }

    if let Some(files) = files {
        for file in files {
            let write_result: anyhow::Result<()> = (|| {
                let rel = validate_template_file_path(&file.path)?;
                let dest = workspace_path.join(rel);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&dest, file.content)?;
                Ok(())
            })();
            if let Err(error) = write_result {
                let _ = std::fs::remove_dir_all(&workspace_path);
                return Err(tauri::ipc::InvokeError::from_anyhow(
                    error.context("Failed to write template files"),
                ));
            }
        }
    }

    let project = Project {
        project_uuid: metadata.project_uuid,
        title: title.to_string(),
        path: clean_path(
            workspace_path
                .canonicalize()
                .context("Failed to resolve created workspace path")
                .map_err(tauri::ipc::InvokeError::from_anyhow)?,
        )
        .to_string_lossy()
        .into_owned(),
        git_branch: detect_git_branch(&workspace_path),
        icon_path: None,
        last_opened_at: Some(unix_timestamp()),
        project_type,
        runtime,
        template,
    };

    record_recent_workspace(project)
}

/// Remove a workspace from the recent list. Does not touch files on disk.
#[tauri::command]
pub async fn remove_recent_workspace(project_uuid: String) -> Result<(), tauri::ipc::InvokeError> {
    let recent_path = recent_workspaces_path()?;
    let mut recent = load_recent_workspaces(&recent_path)?;
    if recent.map.remove(&project_uuid).is_some() {
        save_recent_workspaces(&recent_path, &recent)?;
    }
    Ok(())
}

/// Show a workspace folder in the system file manager.
#[tauri::command]
pub async fn reveal_workspace(
    path: String,
    app_handle: tauri::AppHandle,
) -> Result<(), tauri::ipc::InvokeError> {
    use tauri_plugin_opener::OpenerExt;

    let path = clean_path(PathBuf::from(path));
    if !path.is_dir() {
        return Err(tauri::ipc::InvokeError::from_anyhow(anyhow::anyhow!(
            "Workspace folder does not exist"
        )));
    }
    app_handle
        .opener()
        .reveal_item_in_dir(&path)
        .context("Failed to show the workspace in the file manager")
        .map_err(tauri::ipc::InvokeError::from_anyhow)
}

#[tauri::command]
pub async fn get_active_workspace(
    window: tauri::WebviewWindow,
) -> Result<Option<Project>, tauri::ipc::InvokeError> {
    Ok(workspace_for_window(window.label()))
}

#[tauri::command]
pub async fn webview_ready(
    app_handle: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<(), tauri::ipc::InvokeError> {
    log::info!("webview_ready label={}", window.label());

    let is_splash = window.label() == crate::splash_window::SPLASH_LABEL;

    if !is_splash {
        // Keep real windows hidden until the splash has had its minimum time.
        let handle = app_handle.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || {
            crate::splash_window::hold_until_splash_done(&handle);
        })
        .await;
    }

    window
        .show()
        .context(format!("Failed to show the {} window", window.label()))
        .map_err(tauri::ipc::InvokeError::from_anyhow)?;
    window
        .set_focus()
        .context(format!("Failed to focus the {} window", window.label()))
        .map_err(tauri::ipc::InvokeError::from_anyhow)?;

    if is_splash {
        crate::splash_window::mark_shown();
    } else {
        crate::splash_window::finish(&app_handle);
    }

    Ok(())
}

#[tauri::command]
pub async fn open_settings_window(
    app_handle: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<(), tauri::ipc::InvokeError> {
    let result = settings_window(app_handle, Some(&window))
        .context("Failed to initialize settings window")
        .context("Error opening settings window");

    if let Err(error) = &result {
        log::error!("{error:#}");
    }

    result.map_err(tauri::ipc::InvokeError::from_anyhow)?;

    Ok(())
}

#[tauri::command]
pub async fn open_devtools(window: tauri::WebviewWindow) -> Result<(), tauri::ipc::InvokeError> {
    log::info!("open_devtools label={}", window.label());
    window.open_devtools();
    Ok(())
}

/// Shared logic: open a workspace and show it in the IDE window with the given label.
/// Used both by the `open_workspace` command and by the single-instance callback.
pub(crate) async fn open_workspace_with_label(
    app: &tauri::AppHandle,
    path: &str,
    label: &str,
) -> Result<Project, tauri::ipc::InvokeError> {
    let workspace_path = clean_path(
        PathBuf::from(path)
            .canonicalize()
            .context("Workspace folder does not exist")
            .map_err(tauri::ipc::InvokeError::from_anyhow)?,
    );
    if !workspace_path.is_dir() {
        return Err(tauri::ipc::InvokeError::from_anyhow(anyhow::anyhow!(
            "Selected workspace is not a directory"
        )));
    }

    let config_dir = workspace_path.join(".luma");
    let config_path = config_dir.join("workspace.toml");
    std::fs::create_dir_all(&config_dir)
        .context("Failed to create workspace configuration directory")
        .map_err(tauri::ipc::InvokeError::from_anyhow)?;
    if !config_path.exists() {
        std::fs::write(&config_path, "# Luma workspace configuration\n")
            .context("Failed to initialize workspace configuration")
            .map_err(tauri::ipc::InvokeError::from_anyhow)?;
    }

    let core = luma_core::CORE_API.get().ok_or_else(|| {
        tauri::ipc::InvokeError::from_anyhow(anyhow::anyhow!("Luma core is not initialized"))
    })?;
    core.open_workspace(&config_path)
        .map_err(tauri::ipc::InvokeError::from_anyhow)?;

    let default_title = workspace_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Workspace")
        .to_string();
    let metadata = load_or_initialize_workspace_metadata(&workspace_path, &default_title)?;
    let project = Project {
        project_uuid: metadata.project_uuid,
        title: metadata.title,
        path: workspace_path.to_string_lossy().into_owned(),
        git_branch: detect_git_branch(&workspace_path),
        icon_path: None,
        last_opened_at: Some(unix_timestamp()),
        project_type: metadata.project_type,
        runtime: metadata.runtime,
        template: metadata.template,
    };
    let project = record_recent_workspace(project)?;

    window_workspaces()
        .write()
        .map_err(|error| {
            tauri::ipc::InvokeError::from_anyhow(anyhow::anyhow!(
                "Failed to update active workspace: {error}"
            ))
        })?
        .insert(label.to_string(), project.clone());

    open_main_window(app, &project, label)
        .context("Failed to open IDE window")
        .map_err(tauri::ipc::InvokeError::from_anyhow)?;

    Ok(project)
}

#[tauri::command]
pub async fn open_workspace(
    path: String,
    new_window: Option<bool>,
    app_handle: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Project, tauri::ipc::InvokeError> {
    log::info!("opening workspace {}", path);
    let new_window = new_window.unwrap_or(false);
    let label = target_window_label(&app_handle, &window, new_window);
    open_workspace_with_label(&app_handle, &path, &label).await
}




