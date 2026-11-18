use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::error::AppError;
use crate::state::{AppSettings, AppState};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsGetRequest {
    pub key: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSetRequest {
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Serialize)]
pub struct SettingsGetResponse {
    pub value: serde_json::Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsActionResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn settings_path(dir: &Path) -> PathBuf {
    dir.join("settings.json")
}

pub fn load_settings(dir: &Path) -> AppSettings {
    let path = settings_path(dir);
    fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<AppSettings>(&bytes).ok())
        .unwrap_or_default()
}

fn persist(state: &AppState, settings: &AppSettings) -> Result<(), AppError> {
    let _guard = state
        .settings_write_lock
        .lock()
        .map_err(|_| AppError::new("Settings lock poisoned"))?;

    let path = settings_path(&state.app_data_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_vec_pretty(settings)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

/// Persist whatever is currently in memory. Used by `theme_set`, which owns the
/// `theme` field but shares the same file.
pub fn persist_current(state: &AppState) -> Result<(), AppError> {
    let snapshot = state
        .settings
        .read()
        .map_err(|_| AppError::new("Settings lock poisoned"))?
        .clone();
    persist(state, &snapshot)
}

#[tauri::command]
pub fn settings_get(
    state: State<'_, AppState>,
    data: SettingsGetRequest,
) -> Result<SettingsGetResponse, AppError> {
    let settings = state
        .settings
        .read()
        .map_err(|_| AppError::new("Settings lock poisoned"))?;

    let value = settings
        .value_of(&data.key)
        .ok_or_else(|| AppError::new(format!("Unknown setting '{}'", data.key)))?;

    Ok(SettingsGetResponse { value })
}

#[tauri::command]
pub fn settings_set(
    app: AppHandle,
    state: State<'_, AppState>,
    data: SettingsSetRequest,
) -> SettingsActionResult {
    let outcome = (|| -> Result<(), AppError> {
        {
            let mut settings = state
                .settings
                .write()
                .map_err(|_| AppError::new("Settings lock poisoned"))?;
            settings
                .set_value(&data.key, &data.value)
                .map_err(AppError::new)?;
            persist(&state, &settings)?;
        }

        let _ = app.emit(
            "settings:on-change",
            serde_json::json!({ "key": data.key, "value": data.value }),
        );

        if data.key == "theme" {
            if let Some(preference) = data.value.as_str() {
                super::theme::apply_and_emit(&app, preference);
            }
        }

        Ok(())
    })();

    match outcome {
        Ok(()) => SettingsActionResult {
            success: true,
            error: None,
        },
        Err(err) => SettingsActionResult {
            success: false,
            error: Some(err.error),
        },
    }
}

#[tauri::command]
pub fn settings_reset(app: AppHandle, state: State<'_, AppState>) -> SettingsActionResult {
    let defaults = AppSettings::default();

    let outcome = (|| -> Result<(), AppError> {
        {
            let mut settings = state
                .settings
                .write()
                .map_err(|_| AppError::new("Settings lock poisoned"))?;
            *settings = defaults.clone();
            persist(&state, &settings)?;
        }

        for key in [
            "theme",
            "displayName",
            "density",
            "aiProvider",
            "aiModel",
            "aiBaseUrl",
        ] {
            if let Some(value) = defaults.value_of(key) {
                let _ = app.emit(
                    "settings:on-change",
                    serde_json::json!({ "key": key, "value": value }),
                );
            }
        }

        super::theme::apply_and_emit(&app, &defaults.theme);
        Ok(())
    })();

    match outcome {
        Ok(()) => SettingsActionResult {
            success: true,
            error: None,
        },
        Err(err) => SettingsActionResult {
            success: false,
            error: Some(err.error),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("das-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn settings_survive_a_write_and_read_round_trip() {
        let dir = temp_dir("settings-roundtrip");

        let settings = AppSettings {
            theme: "midnight".into(),
            display_name: "Ada".into(),
            density: "compact".into(),
            ai_provider: "google".into(),
            ai_model: "gemini-2.5-flash".into(),
            ai_base_url: "http://localhost:11434/v1".into(),
        };

        let state = AppState::new(dir.clone(), settings.clone());
        persist(&state, &settings).unwrap();

        let loaded = load_settings(&dir);
        assert_eq!(loaded.theme, "midnight");
        assert_eq!(loaded.display_name, "Ada");
        assert_eq!(loaded.density, "compact");
        assert_eq!(loaded.ai_provider, "google");
        assert_eq!(loaded.ai_model, "gemini-2.5-flash");
        assert_eq!(loaded.ai_base_url, "http://localhost:11434/v1");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_partial_settings_file_fills_in_defaults() {
        let dir = temp_dir("settings-partial");
        fs::write(settings_path(&dir), br#"{"displayName":"Ada"}"#).unwrap();

        let loaded = load_settings(&dir);
        assert_eq!(loaded.display_name, "Ada");
        assert_eq!(loaded.theme, "system");
        assert_eq!(loaded.ai_model, "claude-opus-5-5");
    }

    #[test]
    fn a_missing_settings_file_returns_defaults() {
        let dir = temp_dir("settings-missing");
        let loaded = load_settings(&dir);
        assert_eq!(loaded.display_name, "User");
        assert_eq!(loaded.density, "comfortable");
    }

    #[test]
    fn unknown_keys_and_wrong_types_are_rejected() {
        let mut settings = AppSettings::default();
        assert!(settings.set_value("nope", &serde_json::json!("x")).is_err());
        assert!(settings
            .set_value("displayName", &serde_json::json!(42))
            .is_err());
        assert!(settings
            .set_value("displayName", &serde_json::json!("Ada"))
            .is_ok());
    }
}
