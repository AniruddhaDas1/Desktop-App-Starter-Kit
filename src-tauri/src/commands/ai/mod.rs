// AI provider dispatch.
//
// The API key is read from the OS credential store here and never crosses into
// the webview: the renderer only ever receives text deltas over an event.
mod anthropic;
mod gemini;
mod openai;
mod provider;
mod sse;

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

use crate::commands::secrets::read_secret;
use crate::error::AppError;
use crate::state::AppState;

use provider::{AiProvider, DEFAULT_OPENAI_BASE};

const STREAM_EVENT: &str = "ai:stream-chunk";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiCompleteRequest {
    pub request_id: String,
    pub prompt: String,
    pub system: Option<String>,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiCancelRequest {
    pub request_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiCompleteResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct AiCancelResponse {
    pub success: bool,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct AiUsage {
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AiStreamChunk {
    request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    delta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    done: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<AiUsage>,
}

fn emit_delta(app: &AppHandle, request_id: &str, text: &str) {
    let _ = app.emit(
        STREAM_EVENT,
        AiStreamChunk {
            request_id: request_id.to_string(),
            delta: Some(text.to_string()),
            done: None,
            error: None,
            usage: None,
        },
    );
}

fn emit_done(app: &AppHandle, request_id: &str, usage: AiUsage) {
    let _ = app.emit(
        STREAM_EVENT,
        AiStreamChunk {
            request_id: request_id.to_string(),
            delta: None,
            done: Some(true),
            error: None,
            usage: Some(usage),
        },
    );
}

fn emit_error(app: &AppHandle, request_id: &str, error: &str) {
    let _ = app.emit(
        STREAM_EVENT,
        AiStreamChunk {
            request_id: request_id.to_string(),
            delta: None,
            done: Some(true),
            error: Some(error.to_string()),
            usage: None,
        },
    );
}

fn validate_model(model: &str, provider: AiProvider) -> Result<(), AppError> {
    let length_ok = !model.is_empty() && model.len() <= 128;
    let chars_ok = model.chars().all(|character| {
        character.is_ascii_alphanumeric()
            || matches!(character, '_' | '.' | ':' | '-')
            || (provider != AiProvider::Google && character == '/')
    });

    if length_ok && chars_ok {
        Ok(())
    } else {
        Err(AppError::new(
            "The model id is empty or contains invalid characters.",
        ))
    }
}

async fn run(app: &AppHandle, state: &AppState, data: &AiCompleteRequest) -> Result<(), AppError> {
    let (provider, model, base_url) = {
        let settings = state
            .settings
            .read()
            .map_err(|_| AppError::new("Settings unavailable"))?;

        let provider = AiProvider::parse(&settings.ai_provider)?;
        let model = data
            .model
            .as_deref()
            .map(str::trim)
            .filter(|model| !model.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| {
                if settings.ai_model.trim().is_empty() {
                    provider.default_model().to_string()
                } else {
                    settings.ai_model.clone()
                }
            });
        let base_url = if settings.ai_base_url.trim().is_empty() {
            DEFAULT_OPENAI_BASE.to_string()
        } else {
            settings.ai_base_url.clone()
        };

        (provider, model, base_url)
    };

    validate_model(&model, provider)?;

    let base = if provider == AiProvider::OpenAi {
        Some(openai::resolve_base(&base_url)?)
    } else {
        None
    };

    let api_key = match read_secret(state, provider.secret_key()) {
        Some(key) if !key.trim().is_empty() => key,
        _ if provider.key_required(base.as_deref()) => {
            return Err(AppError::new(provider.missing_key_message()));
        }
        _ => String::new(),
    };

    let token = CancellationToken::new();
    {
        let mut cancels = state
            .cancels
            .lock()
            .map_err(|_| AppError::new("Cancel registry unavailable"))?;
        cancels.insert(data.request_id.clone(), token.clone());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(AppError::from)?;

    let result = match provider {
        AiProvider::Anthropic => {
            anthropic::stream(&client, app, data, &model, &api_key, token).await
        }
        AiProvider::OpenAi => {
            openai::stream(
                &client,
                app,
                data,
                &model,
                base.as_deref().unwrap_or_default(),
                &api_key,
                token,
            )
            .await
        }
        AiProvider::Google => gemini::stream(&client, app, data, &model, &api_key, token).await,
    };

    if let Ok(mut cancels) = state.cancels.lock() {
        cancels.remove(&data.request_id);
    }

    if let Ok(usage) = &result {
        emit_done(app, &data.request_id, usage.clone());
    }

    result.map(|_| ())
}

#[tauri::command]
pub async fn ai_complete(
    app: AppHandle,
    state: State<'_, AppState>,
    data: AiCompleteRequest,
) -> Result<AiCompleteResponse, AppError> {
    match run(&app, &state, &data).await {
        Ok(()) => Ok(AiCompleteResponse {
            success: true,
            error: None,
        }),
        Err(err) => {
            emit_error(&app, &data.request_id, &err.error);
            Ok(AiCompleteResponse {
                success: false,
                error: Some(err.error),
            })
        }
    }
}

#[tauri::command]
pub fn ai_cancel(state: State<'_, AppState>, data: AiCancelRequest) -> AiCancelResponse {
    let cancelled = state
        .cancels
        .lock()
        .ok()
        .and_then(|cancels| cancels.get(&data.request_id).cloned())
        .map(|token| {
            token.cancel();
            true
        })
        .unwrap_or(false);

    AiCancelResponse { success: cancelled }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_ids_allow_slashes_except_for_gemini() {
        assert!(validate_model("anthropic/claude-sonnet-4.5", AiProvider::OpenAi).is_ok());
        assert!(validate_model("claude-opus-5-5", AiProvider::Anthropic).is_ok());
        assert!(validate_model("gemini-2.5-flash", AiProvider::Google).is_ok());
        assert!(validate_model("gemini/../secret", AiProvider::Google).is_err());
        assert!(validate_model("", AiProvider::Anthropic).is_err());
    }
}
