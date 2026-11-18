use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State, Theme};

use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeGetResponse {
    pub preference: String,
    pub resolved: String,
}

#[derive(Deserialize)]
pub struct ThemeSetRequest {
    pub preference: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeSetResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

const VALID_PREFERENCES: [&str; 6] = ["system", "light", "dark", "slate", "midnight", "hc"];

fn window_theme(app: &AppHandle) -> Option<Theme> {
    app.get_webview_window("main")
        .and_then(|window| window.theme().ok())
}

/// `system` resolves against the OS appearance; the other variants are explicit
/// themes the user picked, so they resolve to themselves.
pub fn resolve(app: &AppHandle, preference: &str) -> String {
    match preference {
        "light" => "light".into(),
        "dark" | "slate" | "midnight" | "hc" => preference.into(),
        _ => match window_theme(app) {
            Some(Theme::Light) => "light".into(),
            _ => "dark".into(),
        },
    }
}

fn native_theme(preference: &str) -> Option<Theme> {
    match preference {
        "light" => Some(Theme::Light),
        "dark" | "slate" | "midnight" | "hc" => Some(Theme::Dark),
        _ => None,
    }
}

/// Apply the native window appearance and tell every webview about the change.
pub fn apply_and_emit(app: &AppHandle, preference: &str) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_theme(native_theme(preference));
    }
    let resolved = resolve(app, preference);
    let _ = app.emit(
        "theme:on-change",
        serde_json::json!({ "preference": preference, "resolved": resolved }),
    );
}

#[tauri::command]
pub fn theme_get(app: AppHandle, state: State<'_, AppState>) -> ThemeGetResponse {
    let preference = state
        .settings
        .read()
        .map(|settings| settings.theme.clone())
        .unwrap_or_else(|_| "system".into());
    let resolved = resolve(&app, &preference);
    ThemeGetResponse {
        preference,
        resolved,
    }
}

#[tauri::command]
pub fn theme_set(
    app: AppHandle,
    state: State<'_, AppState>,
    data: ThemeSetRequest,
) -> ThemeSetResponse {
    if !VALID_PREFERENCES.contains(&data.preference.as_str()) {
        return ThemeSetResponse {
            success: false,
            error: Some(format!("Unknown theme '{}'", data.preference)),
        };
    }

    {
        let mut settings = match state.settings.write() {
            Ok(settings) => settings,
            Err(_) => {
                return ThemeSetResponse {
                    success: false,
                    error: Some("Settings lock poisoned".into()),
                }
            }
        };
        settings.theme = data.preference.clone();
    }

    if let Err(err) = super::settings::persist_current(&state) {
        return ThemeSetResponse {
            success: false,
            error: Some(err.error),
        };
    }

    apply_and_emit(&app, &data.preference);
    ThemeSetResponse {
        success: true,
        error: None,
    }
}
