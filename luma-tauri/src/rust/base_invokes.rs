use crate::settings_window::settings_window;
use anyhow::Context;
use luma_api::projects::{Project, RecentProjects};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{OnceLock, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use uuid::Uuid;

static ACTIVE_WORKSPACE: OnceLock<RwLock<Option<Project>>> = OnceLock::new();

pub fn active_workspace() -> &'static RwLock<Option<Project>> {
    ACTIVE_WORKSPACE.get_or_init(|| RwLock::new(None))
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Debug, Serialize, Deserialize)]
struct WorkspaceMetadata {
    project_uuid: String,
    title: String,
    #[serde(default)]
    created_at: u64,
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

fn open_main_window(app: &tauri::AppHandle, project: &Project) -> anyhow::Result<()> {
    let title = format!("Luma — {}", project.title);

    let window = if let Some(window) = app.get_webview_window("main") {
        window.set_title(&title)?;
        window
    } else {
        WebviewWindowBuilder::new(app, "main", WebviewUrl::App("windows/main.html".into()))
            .visible(false)
            .decorations(!cfg!(target_os = "windows"))
            .min_inner_size(750.0, 500.0)
            .title(&title)
            .background_color(crate::STARTUP_COLOR)
            .build()
            .context("Failed to create the IDE window")?
    };

    window.show().context("Failed to show the IDE window")?;
    window
        .set_focus()
        .context("Failed to focus the IDE window")?;

    app.emit("workspace-opened", project.clone())?;
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
pub async fn create_workspace(
    title: String,
    parent_path: String,
) -> Result<Project, tauri::ipc::InvokeError> {
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

    let project = Project {
        project_uuid: metadata.project_uuid,
        title: title.to_string(),
        path: workspace_path
            .canonicalize()
            .context("Failed to resolve created workspace path")
            .map_err(tauri::ipc::InvokeError::from_anyhow)?
            .to_string_lossy()
            .into_owned(),
        git_branch: None,
        icon_path: None,
        last_opened_at: Some(unix_timestamp()),
    };

    record_recent_workspace(project)
}

#[tauri::command]
pub async fn get_active_workspace() -> Result<Option<Project>, tauri::ipc::InvokeError> {
    active_workspace()
        .read()
        .map(|workspace| workspace.clone())
        .map_err(|error| {
            tauri::ipc::InvokeError::from_anyhow(anyhow::anyhow!(
                "Failed to read active workspace: {error}"
            ))
        })
}

#[tauri::command]
pub async fn webview_ready(window: tauri::WebviewWindow) -> Result<(), tauri::ipc::InvokeError> {
    log::info!("webview_ready label={}", window.label());

    window
        .show()
        .context(format!("Failed to show the {} window", window.label()))
        .map_err(tauri::ipc::InvokeError::from_anyhow)?;
    window
        .set_focus()
        .context(format!("Failed to focus the {} window", window.label()))
        .map_err(tauri::ipc::InvokeError::from_anyhow)?;

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

#[tauri::command]
pub async fn open_workspace(
    path: String,
    app_handle: tauri::AppHandle,
) -> Result<(), tauri::ipc::InvokeError> {
    log::info!("opening workspace {}", path);

    let workspace_path = PathBuf::from(path)
        .canonicalize()
        .context("Workspace folder does not exist")
        .map_err(tauri::ipc::InvokeError::from_anyhow)?;
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
        git_branch: None,
        icon_path: None,
        last_opened_at: Some(unix_timestamp()),
    };
    let project = record_recent_workspace(project)?;

    *active_workspace().write().map_err(|error| {
        tauri::ipc::InvokeError::from_anyhow(anyhow::anyhow!(
            "Failed to update active workspace: {error}"
        ))
    })? = Some(project.clone());

    open_main_window(&app_handle, &project)
        .context("Failed to open IDE window")
        .map_err(tauri::ipc::InvokeError::from_anyhow)?;

    Ok(())
}
