use crate::error::AppError;
use serde::Deserialize;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Manager, PhysicalPosition, Position};

#[derive(Deserialize, Default)]
pub struct ShowWindowMenuRequest {
    pub x: Option<f64>,
    pub y: Option<f64>,
}

/// The hamburger menu on Windows/Linux, popped up under the button.
#[tauri::command]
pub fn menu_show_window_menu(
    app: AppHandle,
    data: Option<ShowWindowMenuRequest>,
) -> Result<(), AppError> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::new("Main window not found"))?;

    let settings = MenuItem::with_id(&app, "settings", "Settings", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(&app)?;
    let quit = MenuItem::with_id(&app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(&app, &[&settings, &separator, &quit])?;

    let request = data.unwrap_or_default();
    let position = Position::Physical(PhysicalPosition::new(
        request.x.unwrap_or(0.0) as i32,
        request.y.unwrap_or(0.0) as i32,
    ));

    window
        .popup_menu_at(&menu, position)
        .map_err(|err| AppError::new(err.to_string()))
}
