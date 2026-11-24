use anyhow::Context;
use luma_api::workspaces::{TemplateFile, Workspace, WorkspaceFace, unix_timestamp};
use luma_api::{CORE_API, CoreAPI, Shared};
use std::path::{Path, PathBuf};
use tauri::ipc::InvokeError;
use tauri::{AppHandle, Emitter, WebviewWindow};

use crate::paths;
use crate::windows::{settings, splash};
use crate::workspace::{metadata, recent, store, window};

type SharedCore = &'static Shared<dyn CoreAPI>;

fn core_api() -> Result<SharedCore, InvokeError> {
    CORE_API
        .get()
        .ok_or_else(|| invoke_error("Luma core is not initialized"))
}

fn invoke_error(message: impl std::fmt::Display) -> InvokeError {
    InvokeError::from_anyhow(anyhow::anyhow!("{message}"))
}

#[tauri::command]
pub async fn get_recent_workspaces() -> Result<Vec<WorkspaceFace>, InvokeError> {
    let path = recent::path()?;
    let mut stored = recent::load()?;
    let missing = recent::prune_missing(&mut stored);

    if !missing.is_empty() {
        crate::json::write(&path, &stored).map_err(InvokeError::from_anyhow)?;
    }

    let mut workspaces: Vec<WorkspaceFace> = stored
        .map
        .iter()
        .filter_map(|(id, face)| recent::refresh(id, face))
        .collect();

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
    project_type: Option<String>,
    runtime: Option<String>,
    template: Option<String>,
    files: Option<Vec<TemplateFile>>,
) -> Result<Workspace, InvokeError> {
    let project_type = metadata::validate_choice_id("project type", project_type)?;
    let runtime = metadata::validate_choice_id("runtime", runtime)?;
    let template = metadata::validate_choice_id("template", template)?;
    let title = validate_title(&title)?;

    let parent = paths::canonical_directory(&parent_path, "Selected parent folder does not exist")
        .map_err(InvokeError::from_anyhow)?;
    let workspace_path = parent.join(title);
    std::fs::create_dir(&workspace_path)
        .with_context(|| format!("Could not create workspace {}", workspace_path.display()))
        .map_err(InvokeError::from_anyhow)?;

    let result = (|| -> Result<Workspace, InvokeError> {
        initialize_config_dir(&workspace_path)?;

        let workspace_metadata = metadata::WorkspaceMetadata {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.to_string(),
            created_at: unix_timestamp(),
            project_type: project_type.clone(),
            runtime: runtime.clone(),
            template: template.clone(),
        };
        metadata::write(&workspace_path, &workspace_metadata)?;

        if let Some(files) = files {
            write_template_files(&workspace_path, files)?;
        }

        let root = paths::canonical_directory(
            &paths::display_path(&workspace_path),
            "Failed to resolve created workspace path",
        )
        .map_err(InvokeError::from_anyhow)?;

        let face = WorkspaceFace::new(
            workspace_metadata.id.clone(),
            title,
            paths::display_path(&root),
        )
        .with_logo(luma_api::utils::get_workspace_icon(&root))
        .with_git_branch(recent::detect_git_branch(&root));

        Ok(Workspace::new(
            recent::record(face)?,
            workspace_metadata.project_type,
            workspace_metadata.runtime,
            workspace_metadata.template,
            Some(workspace_metadata.created_at),
        ))
    })();

    if result.is_err() {
        let _ = std::fs::remove_dir_all(&workspace_path);
    }
    result
}

#[tauri::command]
pub async fn remove_recent_workspace(project_uuid: String) -> Result<(), InvokeError> {
    recent::remove(&project_uuid)
}

#[tauri::command]
pub async fn reveal_workspace(path: String, app_handle: AppHandle) -> Result<(), InvokeError> {
    use tauri_plugin_opener::OpenerExt;

    let path = paths::clean_path(PathBuf::from(path));
    if !path.is_dir() {
        return Err(invoke_error("Workspace folder does not exist"));
    }

    app_handle
        .opener()
        .reveal_item_in_dir(&path)
        .context("Failed to show the workspace in the file manager")
        .map_err(InvokeError::from_anyhow)
}

#[tauri::command]
pub async fn get_active_workspace(window: WebviewWindow) -> Result<Option<Workspace>, InvokeError> {
    Ok(store::workspace_for_window(window.label()))
}

#[tauri::command]
pub async fn webview_ready(
    app_handle: AppHandle,
    window: WebviewWindow,
) -> Result<(), InvokeError> {
    log::info!("webview_ready label={}", window.label());

    let is_splash = window.label() == splash::SPLASH_LABEL;

    if !is_splash {
        let handle = app_handle.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || {
            splash::hold_until_splash_done(&handle);
        })
        .await;
    }

    window
        .show()
        .with_context(|| format!("Failed to show the {} window", window.label()))
        .map_err(InvokeError::from_anyhow)?;
    window
        .set_focus()
        .with_context(|| format!("Failed to focus the {} window", window.label()))
        .map_err(InvokeError::from_anyhow)?;

    if is_splash {
        splash::mark_shown();
    } else {
        splash::finish(&app_handle);
    }

    Ok(())
}

#[tauri::command]
pub async fn open_settings_window(
    app_handle: AppHandle,
    window: WebviewWindow,
) -> Result<(), InvokeError> {
    let result = settings::settings_window(app_handle, Some(&window))
        .context("Failed to initialize settings window")
        .context("Error opening settings window");

    if let Err(error) = &result {
        log::error!("{error:#}");
    }
    result.map(|_| ()).map_err(InvokeError::from_anyhow)
}

#[tauri::command]
pub async fn open_devtools(window: WebviewWindow) -> Result<(), InvokeError> {
    log::info!("open_devtools label={}", window.label());
    window.open_devtools();
    Ok(())
}

/// Shared open flow for both the `open_workspace` command and the
/// single-instance callback.
pub(crate) async fn open_workspace_with_label(
    app: &AppHandle,
    path: &str,
    label: &str,
) -> Result<Workspace, InvokeError> {
    let root = paths::canonical_directory(path, "Workspace folder does not exist")
        .map_err(InvokeError::from_anyhow)?;
    initialize_config_dir(&root)?;

    let stored = recent::load()?;
    let default_title = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Workspace")
        .to_string();
    let existing_id =
        recent::existing_for_path(&stored, &paths::display_path(&root)).map(|(id, face)| {
            if face.id.trim().is_empty() {
                id
            } else {
                face.id
            }
        });
    let metadata = metadata::load_or_initialize(&root, &default_title, existing_id)?;

    let face = WorkspaceFace::new(
        metadata.id.clone(),
        metadata.title.clone(),
        paths::display_path(&root),
    )
    .with_logo(recent::logo_for_path(&stored, &root))
    .with_git_branch(recent::detect_git_branch(&root));

    let mut workspace = Workspace::new(
        face,
        metadata.project_type,
        metadata.runtime,
        metadata.template,
        Some(metadata.created_at),
    );

    workspace
        .load(core_api()?.as_ref())
        .map_err(InvokeError::from_anyhow)?;
    workspace.face = recent::record(workspace.face.clone())?;

    store::store().set(label, workspace.clone());
    window::open_main_window(app, &workspace, label)
        .context("Failed to open IDE window")
        .map_err(InvokeError::from_anyhow)?;

    if let Err(error) = app.emit_to(label, "workspace-opened", workspace.face()) {
        log::warn!("Failed to emit workspace-opened to {label}: {error}");
    }

    Ok(workspace)
}

#[tauri::command]
pub async fn open_workspace(
    path: String,
    new_window: Option<bool>,
    app_handle: AppHandle,
    caller: WebviewWindow,
) -> Result<Workspace, InvokeError> {
    log::info!("opening workspace {path}");
    let label = window::target_window_label(&app_handle, &caller, new_window.unwrap_or(false));
    open_workspace_with_label(&app_handle, &path, &label).await
}

fn validate_title(title: &str) -> Result<&str, InvokeError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(invoke_error("Workspace name cannot be empty"));
    }
    if title
        .chars()
        .any(|ch| ch.is_control() || "/\\<>:\"|?*".contains(ch))
        || title.ends_with('.')
        || title.ends_with(' ')
    {
        return Err(invoke_error("Workspace name contains invalid characters"));
    }
    Ok(title)
}

fn initialize_config_dir(root: &Path) -> Result<(), InvokeError> {
    let config_dir = root.join(".luma");
    std::fs::create_dir_all(&config_dir)
        .context("Failed to create workspace configuration directory")
        .map_err(InvokeError::from_anyhow)?;

    let config_path = config_dir.join("workspace.toml");
    if !config_path.exists() {
        std::fs::write(&config_path, "# Luma workspace configuration\n")
            .context("Failed to initialize workspace configuration")
            .map_err(InvokeError::from_anyhow)?;
    }
    Ok(())
}

fn write_template_files(root: &Path, files: Vec<TemplateFile>) -> Result<(), InvokeError> {
    for file in files {
        let relative = metadata::validate_template_file_path(&file.path)
            .context("Failed to write template files")
            .map_err(InvokeError::from_anyhow)?;
        let destination = root.join(relative);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))
                .map_err(InvokeError::from_anyhow)?;
        }
        std::fs::write(&destination, file.content)
            .with_context(|| format!("Failed to write {}", destination.display()))
            .map_err(InvokeError::from_anyhow)?;
    }
    Ok(())
}
