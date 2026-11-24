use tauri::{Manager, Runtime, WebviewWindow, WebviewWindowBuilder};

/// Attach an optional parent window to a builder without duplicating the
/// `Some` / `None` branch at every call site.
pub fn with_parent<'a, R: Runtime, M: Manager<R>>(
    builder: WebviewWindowBuilder<'a, R, M>,
    parent: Option<&WebviewWindow<R>>,
) -> tauri::Result<WebviewWindowBuilder<'a, R, M>> {
    match parent {
        Some(parent) => builder.parent(parent),
        None => Ok(builder),
    }
}
