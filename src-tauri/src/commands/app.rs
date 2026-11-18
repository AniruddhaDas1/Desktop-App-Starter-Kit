use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::commands::fs::is_path_allowed;
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfoResponse {
    pub version: String,
    pub name: String,
    // Field names kept from the Electron contract so the renderer types and the
    // Settings screen keep working; the values are Tauri equivalents.
    pub electron_version: String,
    pub chrome_version: String,
    pub node_version: String,
    pub build_hash: String,
    pub build_date: String,
    pub homepage: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Deserialize)]
pub struct OpenExternalRequest {
    pub url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathRequest {
    pub path: String,
}

#[tauri::command]
pub fn app_get_info(app: AppHandle) -> AppInfoResponse {
    let version = app.package_info().version.to_string();
    let name = app
        .config()
        .product_name
        .clone()
        .unwrap_or_else(|| app.package_info().name.clone());

    AppInfoResponse {
        version,
        name,
        electron_version: format!("tauri {}", tauri::VERSION),
        chrome_version: tauri::webview_version().unwrap_or_else(|_| "unknown".into()),
        node_version: env!("RUSTC_VERSION").to_string(),
        build_hash: option_env!("GIT_HASH").unwrap_or("dev").to_string(),
        build_date: env!("BUILD_TIMESTAMP").to_string(),
        homepage: non_empty(option_env!("CARGO_PKG_HOMEPAGE")),
        description: non_empty(option_env!("CARGO_PKG_DESCRIPTION")),
    }
}

fn non_empty(value: Option<&'static str>) -> Option<String> {
    value.filter(|v| !v.is_empty()).map(|v| v.to_string())
}

#[tauri::command]
pub fn app_open_external(data: OpenExternalRequest) -> ActionResult {
    // Only hand web URLs to the OS opener; anything else could launch a
    // registered protocol handler the user did not intend.
    if !(data.url.starts_with("https://") || data.url.starts_with("http://")) {
        return ActionResult {
            success: false,
            error: Some("Only http(s) URLs can be opened externally".into()),
        };
    }

    match tauri_plugin_opener::open_url(data.url.as_str(), None::<&str>) {
        Ok(()) => ActionResult {
            success: true,
            error: None,
        },
        Err(err) => ActionResult {
            success: false,
            error: Some(err.to_string()),
        },
    }
}

/// Extensions that run code instead of opening a viewer. A renderer with write
/// access to a blessed folder could otherwise write a binary and open it.
const DANGEROUS_OPEN_PATH_EXTENSIONS: [&str; 24] = [
    "exe", "bat", "cmd", "com", "ps1", "psm1", "scr", "msi", "msp", "app", "command", "sh", "bash",
    "zsh", "lnk", "jar", "appimage", "vbs", "vbe", "js", "jse", "wsf", "pif", "dmg",
];

fn is_dangerous_extension(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| DANGEROUS_OPEN_PATH_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

#[tauri::command]
pub fn app_open_path(state: State<'_, AppState>, data: PathRequest) -> ActionResult {
    // Same allow-list as fs:* — otherwise the renderer could use the OS opener
    // to reach paths it was never granted.
    if !is_path_allowed(&state, &data.path) {
        return ActionResult {
            success: false,
            error: Some("Path is outside the allow-list".into()),
        };
    }

    if is_dangerous_extension(&data.path) {
        return ActionResult {
            success: false,
            error: Some("Path extension is not permitted".into()),
        };
    }

    match tauri_plugin_opener::open_path(data.path.as_str(), None::<&str>) {
        Ok(()) => ActionResult {
            success: true,
            error: None,
        },
        Err(err) => ActionResult {
            success: false,
            error: Some(err.to_string()),
        },
    }
}

#[tauri::command]
pub fn app_show_in_folder(state: State<'_, AppState>, data: PathRequest) -> ActionResult {
    if !is_path_allowed(&state, &data.path) {
        return ActionResult {
            success: false,
            error: Some("Path is outside the allow-list".into()),
        };
    }

    match tauri_plugin_opener::reveal_item_in_dir(data.path.as_str()) {
        Ok(()) => ActionResult {
            success: true,
            error: None,
        },
        Err(err) => ActionResult {
            success: false,
            error: Some(err.to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_extensions_are_rejected() {
        assert!(is_dangerous_extension("/tmp/setup.exe"));
        assert!(is_dangerous_extension("/tmp/run.SH"));
        assert!(is_dangerous_extension("a.b.command"));
        assert!(is_dangerous_extension("/tmp/App.jar"));
        assert!(is_dangerous_extension("/tmp/thing.dmg"));
    }

    #[test]
    fn ordinary_files_are_allowed() {
        assert!(!is_dangerous_extension("/tmp/notes.txt"));
        assert!(!is_dangerous_extension("/tmp/photo.png"));
        assert!(!is_dangerous_extension("/tmp/no-extension"));
    }
}
