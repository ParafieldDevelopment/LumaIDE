use anyhow::Context;
use crate::settings_window::settings_window;

#[tauri::command]
pub async fn get_recent_workspaces() -> Result<Vec<luma_api::projects::Project>, tauri::ipc::InvokeError> {
    let core = luma_core::CORE_API
        .get()
        .ok_or_else(|| tauri::ipc::InvokeError::from_anyhow(anyhow::anyhow!("Luma core is not initialized")))?;
    let data_dir = core.get_data_dir();
    let recent_workspaces_path = data_dir.join("recent_workspaces.json");

    let contents = match std::fs::read_to_string(&recent_workspaces_path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(tauri::ipc::InvokeError::from_anyhow(
                anyhow::Error::from(error).context(format!(
                    "Failed to read recent workspaces from {}",
                    recent_workspaces_path.display()
                )),
            ));
        }
    };

    let mut recent_workspaces: luma_api::projects::RecentProjects =
        serde_json::from_str(&contents)
            .context(format!(
                "Failed to parse recent workspaces from {}",
                recent_workspaces_path.display()
            ))
            .map_err(tauri::ipc::InvokeError::from_anyhow)?;

    Ok(recent_workspaces
        .map
        .drain()
        .map(|(project_uuid, mut project)| {
            project.project_uuid = project_uuid;
            project.icon_path = project.icon_path.map(|icon_path| {
                let icon_path = std::path::PathBuf::from(icon_path);
                if icon_path.is_absolute() {
                    icon_path
                } else {
                    data_dir.join(icon_path)
                }
                .to_string_lossy()
                .into_owned()
            });
            project
        })
        .collect())
}

#[tauri::command]
pub async fn webview_ready(window: tauri::WebviewWindow) -> Result<(), tauri::ipc::InvokeError> {
    log::info!("webview_ready label={}", window.label());

    window.show()
        .context(format!("Failed to show the {} window", window.label()))
        .map_err(tauri::ipc::InvokeError::from_anyhow)?;
    window.set_focus()
        .context(format!("Failed to focus the {} window", window.label()))
        .map_err(tauri::ipc::InvokeError::from_anyhow)?;

    Ok(())
}
#[tauri::command]
pub async fn open_settings_window(app_handle: tauri::AppHandle,window: tauri::WebviewWindow) -> Result<(), tauri::ipc::InvokeError> {
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
pub async fn open_workspace(path: String) -> Result<(), tauri::ipc::InvokeError> {
    log::info!("opening project {}", path);

    let workspace_path = std::path::PathBuf::from(path);

    if let Some(core) = luma_core::CORE_API.get() {
        core.open_workspace(&workspace_path)
            .map_err(tauri::ipc::InvokeError::from_anyhow)?;
    }

    // TODO: load configurations window (frontend will handle UI).

    Ok(())
}