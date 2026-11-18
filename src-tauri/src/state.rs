use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};

use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

/// Persisted user settings. Field names are camelCase on the wire so the
/// renderer's `AppSettings` type (a Zod schema) describes the same object.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub theme: String,
    pub display_name: String,
    pub density: String,
    pub ai_provider: String,
    pub ai_model: String,
    pub ai_base_url: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            display_name: "User".into(),
            density: "comfortable".into(),
            ai_provider: "anthropic".into(),
            ai_model: "claude-opus-5-5".into(),
            ai_base_url: "https://api.openai.com/v1".into(),
        }
    }
}

impl AppSettings {
    /// Read one setting as JSON, matching the renderer's `SettingsGetResponse`.
    pub fn value_of(&self, key: &str) -> Option<serde_json::Value> {
        match key {
            "theme" => Some(serde_json::json!(self.theme)),
            "displayName" => Some(serde_json::json!(self.display_name)),
            "density" => Some(serde_json::json!(self.density)),
            "aiProvider" => Some(serde_json::json!(self.ai_provider)),
            "aiModel" => Some(serde_json::json!(self.ai_model)),
            "aiBaseUrl" => Some(serde_json::json!(self.ai_base_url)),
            _ => None,
        }
    }

    /// Apply one setting, rejecting unknown keys and wrong value types.
    pub fn set_value(&mut self, key: &str, value: &serde_json::Value) -> Result<(), String> {
        let as_str = |v: &serde_json::Value| {
            v.as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| format!("Setting '{key}' expects a string"))
        };
        match key {
            "theme" => self.theme = as_str(value)?,
            "displayName" => self.display_name = as_str(value)?,
            "density" => self.density = as_str(value)?,
            "aiProvider" => self.ai_provider = as_str(value)?,
            "aiModel" => self.ai_model = as_str(value)?,
            "aiBaseUrl" => self.ai_base_url = as_str(value)?,
            _ => return Err(format!("Unknown setting '{key}'")),
        }
        Ok(())
    }
}

pub struct AppState {
    /// `app_data_dir()` — always readable through `fs:*`, like Electron's userData.
    pub app_data_dir: PathBuf,
    /// Paths the user picked or dropped. Children are allowed too.
    pub blessed: Mutex<HashSet<PathBuf>>,
    pub settings: RwLock<AppSettings>,
    /// Serializes settings writes so two commands cannot interleave tmp+rename.
    pub settings_write_lock: Mutex<()>,
    /// Same lock for the plaintext secret fallback file.
    pub secrets_write_lock: Mutex<()>,
    /// In-flight AI requests, keyed by requestId, for `ai_cancel`.
    pub cancels: Mutex<HashMap<String, CancellationToken>>,
}

impl AppState {
    pub fn new(app_data_dir: PathBuf, settings: AppSettings) -> Self {
        // Canonicalize the app data root once: the fs allow-list compares
        // canonical paths, and on macOS the temp/home tree can sit behind a
        // symlink (e.g. /var -> /private/var), which would otherwise make every
        // app-data path look like it is outside the allow-list.
        let app_data_dir = std::fs::canonicalize(&app_data_dir).unwrap_or(app_data_dir);

        Self {
            app_data_dir,
            blessed: Mutex::new(HashSet::new()),
            settings: RwLock::new(settings),
            settings_write_lock: Mutex::new(()),
            secrets_write_lock: Mutex::new(()),
            cancels: Mutex::new(HashMap::new()),
        }
    }
}
