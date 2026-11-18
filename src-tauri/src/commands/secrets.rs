// Port of src/main/services/secret-store.ts.
//
// Secrets live in the OS credential store (Keychain / Credential Manager /
// Secret Service) through the `keyring` crate. No secret is ever returned to the
// renderer: `secret_has` reports presence plus a masked preview only.
//
// If no backend is available (headless Linux, CI), we fall back to the same
// plaintext file the Electron starter used and report
// `encryptionAvailable: false` so the AI screen can warn the user.
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

const KEYRING_SERVICE: &str = "desktop-app-starter-kit-tauri";
const FALLBACK_FILENAME: &str = "secrets.dat";

#[derive(Deserialize)]
pub struct SecretKeyRequest {
    pub key: String,
}

#[derive(Deserialize)]
pub struct SecretSetRequest {
    pub key: String,
    pub value: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretHasResponse {
    pub present: bool,
    pub preview: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretActionResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretStatusResponse {
    pub encryption_available: bool,
    pub platform: String,
}

fn entry(key: &str) -> Result<keyring::Entry, AppError> {
    keyring::Entry::new(KEYRING_SERVICE, key)
        .map_err(|err| AppError::new(format!("Keyring unavailable: {err}")))
}

fn id(key: &str) -> String {
    format!("{KEYRING_SERVICE}/{key}")
}

/// True when the platform credential store accepts operations.
fn encryption_available() -> bool {
    match keyring::Entry::new(KEYRING_SERVICE, "__probe__") {
        Ok(probe) => match probe.get_password() {
            Ok(_) => true,
            Err(keyring::Error::NoEntry) => true,
            Err(_) => false,
        },
        Err(_) => false,
    }
}

fn fallback_path(state: &AppState) -> PathBuf {
    state.app_data_dir.join(FALLBACK_FILENAME)
}

fn read_fallback(state: &AppState) -> HashMap<String, String> {
    fs::read(fallback_path(state))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<HashMap<String, String>>(&bytes).ok())
        .unwrap_or_default()
}

fn write_fallback(state: &AppState, map: &HashMap<String, String>) -> Result<(), AppError> {
    let _guard = state
        .secrets_write_lock
        .lock()
        .map_err(|_| AppError::new("Secret lock poisoned"))?;

    let path = fallback_path(state);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec(map)?;
    fs::write(&path, json)?;
    Ok(())
}

fn mask(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 8 {
        return "•".repeat(chars.len().clamp(1, 8));
    }
    let head: String = chars[..4].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}…{tail}")
}

/// Read a secret for internal use (the AI commands). Renderer never sees this.
pub fn read_secret(state: &AppState, key: &str) -> Option<String> {
    match entry(key) {
        Ok(entry) => match entry.get_password() {
            Ok(value) => Some(value),
            Err(keyring::Error::NoEntry) => None,
            Err(_) => read_fallback(state).get(&id(key)).cloned(),
        },
        Err(_) => read_fallback(state).get(&id(key)).cloned(),
    }
}

#[tauri::command]
pub fn secret_has(state: State<'_, AppState>, data: SecretKeyRequest) -> SecretHasResponse {
    let value = read_secret(&state, &data.key);
    SecretHasResponse {
        present: value.is_some(),
        preview: value.as_deref().map(mask),
    }
}

fn store_secret(state: &AppState, key: &str, value: &str) -> Result<(), AppError> {
    if let Ok(keyring_entry) = entry(key) {
        if keyring_entry.set_password(value).is_ok() {
            return Ok(());
        }
    }

    let mut map = read_fallback(state);
    map.insert(id(key), value.to_string());
    write_fallback(state, &map)
}

fn remove_secret(state: &AppState, key: &str) -> Result<(), AppError> {
    if let Ok(keyring_entry) = entry(key) {
        let _ = keyring_entry.delete_credential();
    }

    let mut map = read_fallback(state);
    if map.remove(&id(key)).is_some() {
        write_fallback(state, &map)?;
    }
    Ok(())
}

#[tauri::command]
pub fn secret_set(state: State<'_, AppState>, data: SecretSetRequest) -> SecretActionResult {
    match store_secret(&state, &data.key, &data.value) {
        Ok(()) => SecretActionResult {
            success: true,
            error: None,
        },
        Err(err) => SecretActionResult {
            success: false,
            error: Some(err.error),
        },
    }
}

#[tauri::command]
pub fn secret_delete(state: State<'_, AppState>, data: SecretKeyRequest) -> SecretActionResult {
    match remove_secret(&state, &data.key) {
        Ok(()) => SecretActionResult {
            success: true,
            error: None,
        },
        Err(err) => SecretActionResult {
            success: false,
            error: Some(err.error),
        },
    }
}

#[tauri::command]
pub fn secret_status() -> SecretStatusResponse {
    SecretStatusResponse {
        encryption_available: encryption_available(),
        platform: std::env::consts::OS.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_never_returns_the_full_secret() {
        let secret = "sk-ant-api03-abcdefghijklmnop";
        let masked = mask(secret);
        assert_ne!(masked, secret);
        assert!(masked.contains('…'));
        assert!(!masked.contains("abcdefghij"));
    }

    #[test]
    fn mask_handles_short_values() {
        assert_eq!(mask("abc"), "•••");
        assert_eq!(mask(""), "•");
    }
}
