use reqwest::Client;
use serde_json::{json, Value};
use tauri::AppHandle;
use tokio_util::sync::CancellationToken;

use super::{emit_delta, sse, AiCompleteRequest, AiUsage};
use crate::error::AppError;

const ANTHROPIC_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u32 = 16000;

pub async fn stream(
    client: &Client,
    app: &AppHandle,
    data: &AiCompleteRequest,
    model: &str,
    api_key: &str,
    token: CancellationToken,
) -> Result<AiUsage, AppError> {
    let response = client
        .post(ANTHROPIC_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .json(&request_body(model, data))
        .send()
        .await
        .map_err(AppError::from)?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::new(format!(
            "Anthropic API error ({status}): {body}"
        )));
    }

    let mut usage = AiUsage {
        model: model.to_string(),
        input_tokens: 0,
        output_tokens: 0,
    };

    sse::for_each_event(response, token, |event| {
        for delta in parse_event(event, &mut usage)? {
            emit_delta(app, &data.request_id, &delta);
        }
        Ok(())
    })
    .await?;

    Ok(usage)
}

fn supports_effort(model: &str) -> bool {
    model.starts_with("claude-opus") || model.starts_with("claude-sonnet")
}

fn request_body(model: &str, data: &AiCompleteRequest) -> Value {
    let mut body = json!({
        "model": model,
        "max_tokens": data.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
        "messages": [{ "role": "user", "content": data.prompt }],
        "stream": true,
    });

    if let Some(system) = data.system.as_deref() {
        body["system"] = json!(system);
    }
    if supports_effort(model) {
        body["output_config"] = json!({ "effort": "medium" });
    }

    body
}

fn parse_event(event: &str, usage: &mut AiUsage) -> Result<Vec<String>, AppError> {
    let mut deltas = Vec::new();

    for line in event.lines() {
        let line = line.trim_end_matches('\r');
        let Some(payload) = line.strip_prefix("data:") else {
            continue;
        };
        let payload = payload.trim();
        if payload.is_empty() || payload == "[DONE]" {
            continue;
        }

        let Ok(value) = serde_json::from_str::<Value>(payload) else {
            continue;
        };

        match value.get("type").and_then(Value::as_str) {
            Some("message_start") => {
                if let Some(model) = value["message"]["model"].as_str() {
                    usage.model = model.to_string();
                }
                if let Some(tokens) = value["message"]["usage"]["input_tokens"].as_u64() {
                    usage.input_tokens = tokens as u32;
                }
            }
            Some("content_block_delta") => {
                if value["delta"]["type"].as_str() == Some("text_delta") {
                    if let Some(text) = value["delta"]["text"].as_str() {
                        deltas.push(text.to_string());
                    }
                }
            }
            Some("message_delta") => {
                if let Some(tokens) = value["usage"]["output_tokens"].as_u64() {
                    usage.output_tokens = tokens as u32;
                }
            }
            Some("error") => {
                let message = value["error"]["message"]
                    .as_str()
                    .unwrap_or("Claude returned an error")
                    .to_string();
                return Err(AppError::new(message));
            }
            _ => {}
        }
    }

    Ok(deltas)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> AiCompleteRequest {
        AiCompleteRequest {
            request_id: "req_test".into(),
            prompt: "hello".into(),
            system: None,
            model: None,
            max_tokens: None,
        }
    }

    #[test]
    fn request_body_matches_the_upstream_defaults() {
        let body = request_body("claude-opus-5-5", &request());
        assert_eq!(body["model"], "claude-opus-5-5");
        assert_eq!(body["max_tokens"], 16000);
        assert_eq!(body["stream"], true);
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"], "hello");
        assert!(body.get("system").is_none());
    }

    #[test]
    fn system_and_max_tokens_are_honoured() {
        let mut data = request();
        data.system = Some("be brief".into());
        data.max_tokens = Some(64);

        let body = request_body("claude-opus-5-5", &data);
        assert_eq!(body["system"], "be brief");
        assert_eq!(body["max_tokens"], 64);
    }

    #[test]
    fn effort_is_only_sent_to_models_that_support_it() {
        assert!(request_body("claude-opus-5-5", &request())
            .get("output_config")
            .is_some());
        assert!(request_body("claude-sonnet-5-5", &request())
            .get("output_config")
            .is_some());
        assert!(request_body("claude-haiku-4-5", &request())
            .get("output_config")
            .is_none());
    }

    #[test]
    fn stream_events_yield_text_and_usage() {
        let mut usage = AiUsage {
            model: "claude-opus-5-5".into(),
            ..Default::default()
        };

        let start = r#"data: {"type":"message_start","message":{"model":"claude-sonnet-5-5","usage":{"input_tokens":12}}}"#;
        let delta =
            r#"data: {"type":"content_block_delta","delta":{"type":"text_delta","text":"Hi"}}"#;
        let stop = r#"data: {"type":"message_delta","usage":{"output_tokens":3}}"#;

        assert!(parse_event(start, &mut usage).unwrap().is_empty());
        assert_eq!(parse_event(delta, &mut usage).unwrap(), vec!["Hi"]);
        assert!(parse_event(stop, &mut usage).unwrap().is_empty());
        assert_eq!(usage.model, "claude-sonnet-5-5");
        assert_eq!(usage.input_tokens, 12);
        assert_eq!(usage.output_tokens, 3);
    }

    #[test]
    fn error_events_become_errors() {
        let mut usage = AiUsage::default();
        let event = r#"data: {"type":"error","error":{"message":"overloaded"}}"#;
        assert_eq!(
            parse_event(event, &mut usage).unwrap_err().error,
            "overloaded"
        );
    }
}
