//! Tauri application builder and plugin wiring.

use anyhow::{Context, Result};
use std::path::Path;
use tauri_plugin_prevent_default::Flags;
use tauri_plugin_window_state::StateFlags;

use crate::windows::{splash, startup};
use crate::workspace;

pub fn build_tauri_app() -> Result<tauri::App> {
    // Store window state in the shared Roaming/Luma directory instead of the
    // bundle-identifier directory Tauri would use by default.
    let window_state_path = luma_core::data_dir()
        .context("Failed to resolve the Luma data directory")?
        .join(".window-state.json");

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_decorum::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_os::init())
        .plugin(
            tauri_plugin_prevent_default::Builder::new()
                .with_flags(Flags::all() & !Flags::CONTEXT_MENU & !Flags::DEV_TOOLS)
                .build(),
        )
        .plugin(tauri_plugin_single_instance::init(
            move |app, args, _cwd| {
                // A second instance was launched (e.g. opening a folder with LumaIDE).
                // Forward the folder to the running instance instead of starting a
                // second copy of the IDE.
                let Some(path) = args
                    .into_iter()
                    .skip(1)
                    .find(|arg| !arg.starts_with('-') && !arg.is_empty())
                else {
                    return;
                };

                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if !Path::new(&path).is_dir() {
                        return;
                    }
                    let label = workspace::free_main_label(&app);
                    let _ =
                        workspace::commands::open_workspace_with_label(&app, &path, &label).await;
                });
            },
        ))
        .plugin(
            tauri_plugin_window_state::Builder::new()
                // DECORATIONS would re-apply a saved `decorations: true` on
                // windows intentionally built with `.decorations(false)`.
                .with_state_flags(
                    StateFlags::all() & !StateFlags::DECORATIONS & !StateFlags::VISIBLE,
                )
                // `Path::join` replaces the base when given an absolute path,
                // so this stores state in Roaming/Luma rather than the bundle id dir.
                .with_filename(window_state_path.to_string_lossy().into_owned())
                .with_denylist(&[splash::SPLASH_LABEL])
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            crate::workspace::commands::webview_ready,
            crate::workspace::commands::open_settings_window,
            crate::workspace::commands::open_devtools,
            crate::workspace::commands::open_workspace,
            crate::workspace::commands::create_workspace,
            crate::workspace::commands::get_active_workspace,
            crate::workspace::commands::get_recent_workspaces,
            crate::workspace::layout::load_tool_window_layout,
            crate::workspace::layout::save_tool_window_layout,
            crate::workspace::commands::remove_recent_workspace,
            crate::workspace::commands::reveal_workspace,
            crate::workspace::commands::report_active_file,
            crate::workspace::commands::report_editor_state,
            crate::discord_rpc::get_discord_rpc_config,
            crate::discord_rpc::set_discord_rpc_config,
            crate::discord_rpc::get_discord_rpc_enabled,
            crate::discord_rpc::set_discord_rpc_enabled,
            crate::filesystem::read_file,
            crate::filesystem::read_file_snapshot,
            crate::filesystem::write_file,
            crate::filesystem::read_directory,
            crate::filesystem::read_directory_tree,
            crate::filesystem::create_file,
            crate::filesystem::create_directory,
            crate::filesystem::delete_entry,
            crate::terminal::git::git_command,
            crate::terminal::sessions::terminal_open_session,
            crate::terminal::sessions::terminal_list_sessions,
            crate::terminal::sessions::terminal_write,
            crate::terminal::sessions::terminal_resize,
            crate::terminal::sessions::terminal_read,
            crate::terminal::sessions::terminal_close_session,
            crate::terminal::sessions::terminal_change_dir,
        ])
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                )])
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            log::info!("Tauri ready");

            // Reset any leftover editor presence so Discord RPC defaults to "Idle"
            // while the user is in the project selection screen.
            let _ = crate::workspace::commands::write_editor_presence(false, None, None, None);

            // The splash is fixed-size; the startup window stays hidden until
            // `webview_ready` has waited out the splash's minimum visible time.
            splash::splash_window(app.handle().clone())
                .context("Failed to initialize splash window")?;
            startup::startup_window(app.handle().clone())
                .context("Failed to initialize startup window")?;

            Ok(())
        })
        .build(tauri::generate_context!())
        .context("Failed to build the Tauri application")?;

    Ok(app)
}
