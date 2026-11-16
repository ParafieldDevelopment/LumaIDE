use std::path::Path;

pub fn get_workspace_icon(workspace_path: &Path) -> Option<String> {
    let luma_dir = workspace_path.join(".luma");

    for extension in ["png", "jpg", "jpeg", "webp", "ico"] {
        let icon = luma_dir.join(format!("icon.{extension}"));

        if icon.is_file() {
            return Some(icon.to_string_lossy().into_owned());
        }
    }

    None
}