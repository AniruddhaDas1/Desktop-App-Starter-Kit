use serde::{Deserialize, Serialize};
use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::commands::fs::bless;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DialogFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DialogOpenFileRequest {
    pub title: Option<String>,
    pub filters: Option<Vec<DialogFilter>>,
    pub multi_selections: Option<bool>,
    pub directory: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogOpenFileResponse {
    pub file_paths: Vec<String>,
    pub canceled: bool,
}

/// Native picker. Picked paths are blessed inside the same command, so the
/// privilege grant and the pick cannot be separated.
#[tauri::command]
pub fn dialog_open_file(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    data: DialogOpenFileRequest,
) -> Result<DialogOpenFileResponse, AppError> {
    let mut dialog = app.dialog().file();

    if let Some(title) = data.title.as_deref() {
        dialog = dialog.set_title(title);
    }

    for filter in data.filters.unwrap_or_default() {
        // "*" means "everything" — the OS picker treats it as no filter.
        if filter.extensions.iter().any(|ext| ext == "*") {
            continue;
        }
        let extensions: Vec<&str> = filter.extensions.iter().map(String::as_str).collect();
        dialog = dialog.add_filter(&filter.name, &extensions);
    }

    let picked = if data.directory.unwrap_or(false) {
        dialog
            .blocking_pick_folder()
            .into_iter()
            .collect::<Vec<_>>()
    } else if data.multi_selections.unwrap_or(false) {
        dialog.blocking_pick_files().unwrap_or_default()
    } else {
        dialog.blocking_pick_file().into_iter().collect::<Vec<_>>()
    };

    let mut paths = Vec::new();
    for file_path in picked {
        if let Ok(path) = file_path.into_path() {
            bless(&state, &path);
            paths.push(path.to_string_lossy().to_string());
        }
    }

    Ok(DialogOpenFileResponse {
        canceled: paths.is_empty(),
        file_paths: paths,
    })
}
