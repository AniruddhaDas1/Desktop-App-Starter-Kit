use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::error::AppError;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowIsMaximizedResponse {
    pub is_maximized: bool,
}

fn main_window(app: &AppHandle) -> Result<WebviewWindow, AppError> {
    app.get_webview_window("main")
        .ok_or_else(|| AppError::new("Main window not found"))
}

#[tauri::command]
pub fn window_minimize(app: AppHandle) -> Result<(), AppError> {
    main_window(&app)?.minimize().map_err(AppError::from)
}

/// Toggles, so the titlebar's Maximize/Restore button is one command.
#[tauri::command]
pub fn window_maximize(app: AppHandle) -> Result<(), AppError> {
    let window = main_window(&app)?;
    let is_maximized = window.is_maximized().unwrap_or(false);
    if is_maximized {
        window.unmaximize().map_err(AppError::from)
    } else {
        window.maximize().map_err(AppError::from)
    }
}

#[tauri::command]
pub fn window_close(app: AppHandle) -> Result<(), AppError> {
    main_window(&app)?.close().map_err(AppError::from)
}

#[tauri::command]
pub fn window_is_maximized(app: AppHandle) -> Result<WindowIsMaximizedResponse, AppError> {
    let window = main_window(&app)?;
    Ok(WindowIsMaximizedResponse {
        is_maximized: window.is_maximized().unwrap_or(false),
    })
}
