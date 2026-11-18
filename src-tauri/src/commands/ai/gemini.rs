use reqwest::Client;
use serde_json::{json, Value};
use tauri::AppHandle;
use tokio_util::sync::CancellationToken;

use super::{emit_delta, sse, AiCompleteRequest, AiUsage};
use crate::error::AppError;

const GEMINI_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

fn stream_url(model: &str) -> String {
    format!("{GEMINI_BASE}/{model}:streamGenerateContent?alt=sse")
}

fn request_body(data: &AiCompleteRequest) -> Value {
    let mut body = json!({
        "contents": [{ "role": "user", "parts": [{ "text": data.prompt }] }],
    });

    if let Some(system) = data.system.as_deref() {
        body["systemInstruction"] = json!({ "parts": [{ "text": system }] });
    }
    if let Some(max_tokens) = data.max_tokens {
        body["generationConfig"] = json!({ "maxOutputTokens": max_tokens });
    }

    body
}

pub async fn stream(
    client: &Client,
    app: &AppHandle,
    data: &AiCompleteRequest,
    model: &str,
    api_key: &str,
    token: CancellationToken,
) -> Result<AiUsage, AppError> {
    let response = client
        .post(stream_url(model))
        .header("x-goog-api-key", api_key)
        .header("content-type", "application/json")
        .json(&request_body(data))
        .send()
        .await
        .map_err(AppError::from)?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::new(format!(
            "Gemini API error ({status}): {body}"
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

        if let Some(message) = value["error"]["message"].as_str() {
            return Err(AppError::new(message.to_string()));
        }
        if let Some(reason) = value["promptFeedback"]["blockReason"].as_str() {
            return Err(AppError::new(format!(
                "Gemini blocked the request: {reason}"
            )));
        }

        if let Some(parts) = value["candidates"][0]["content"]["parts"].as_array() {
            for part in parts {
                if let Some(text) = part["text"].as_str() {
                    deltas.push(text.to_string());
                }
            }
        }

        if let Some(tokens) = value["usageMetadata"]["promptTokenCount"].as_u64() {
            usage.input_tokens = tokens as u32;
        }
        if let Some(tokens) = value["usageMetadata"]["candidatesTokenCount"].as_u64() {
            usage.output_tokens = tokens as u32;
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
    fn request_body_uses_contents_and_system_instruction() {
        let mut data = request();
        data.system = Some("be brief".into());
        data.max_tokens = Some(64);

        let body = request_body(&data);
        assert_eq!(body["contents"][0]["role"], "user");
        assert_eq!(body["contents"][0]["parts"][0]["text"], "hello");
        assert_eq!(body["systemInstruction"]["parts"][0]["text"], "be brief");
        assert_eq!(body["generationConfig"]["maxOutputTokens"], 64);
    }

    #[test]
    fn stream_url_targets_the_streaming_endpoint() {
        assert_eq!(
            stream_url("gemini-2.5-flash"),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:streamGenerateContent?alt=sse"
        );
    }

    #[test]
    fn stream_events_yield_text_and_usage() {
        let mut usage = AiUsage {
            model: "gemini-2.5-flash".into(),
            ..Default::default()
        };

        let chunk = r#"data: {"candidates":[{"content":{"parts":[{"text":"Hi"},{"text":" there"}]}}],"usageMetadata":{"promptTokenCount":7,"candidatesTokenCount":2}}"#;
        assert_eq!(
            parse_event(chunk, &mut usage).unwrap(),
            vec!["Hi", " there"]
        );
        assert_eq!(usage.input_tokens, 7);
        assert_eq!(usage.output_tokens, 2);
    }

    #[test]
    fn blocked_requests_become_errors() {
        let mut usage = AiUsage::default();
        let event = r#"data: {"promptFeedback":{"blockReason":"SAFETY"}}"#;
        assert!(parse_event(event, &mut usage)
            .unwrap_err()
            .error
            .contains("SAFETY"));
    }
}
