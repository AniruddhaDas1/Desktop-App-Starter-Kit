use reqwest::Client;
use serde_json::{json, Value};
use tauri::AppHandle;
use tokio_util::sync::CancellationToken;

use super::provider::is_local_host;
use super::{emit_delta, sse, AiCompleteRequest, AiUsage};
use crate::error::AppError;

/// Validate the user-supplied API root. Plain http is allowed only for local
/// endpoints so a remote key is never sent in the clear.
pub fn resolve_base(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(AppError::new("The OpenAI-compatible base URL is empty."));
    }

    let parsed = reqwest::Url::parse(trimmed)
        .map_err(|_| AppError::new("The OpenAI-compatible base URL is not valid."))?;

    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err(AppError::new(
            "The base URL must not include a query string or fragment.",
        ));
    }

    match parsed.scheme() {
        "https" => {}
        "http" if is_local_host(trimmed) => {}
        "http" => {
            return Err(AppError::new(
                "Use https:// for a remote base URL. Plain http is allowed only for localhost.",
            ));
        }
        scheme => {
            return Err(AppError::new(format!(
                "Unsupported base URL scheme '{scheme}'. Use http or https."
            )));
        }
    }

    Ok(trimmed.to_string())
}

fn completions_url(base: &str) -> String {
    if base.ends_with("/chat/completions") {
        base.to_string()
    } else {
        format!("{base}/chat/completions")
    }
}

fn is_openai_api(base: &str) -> bool {
    reqwest::Url::parse(base)
        .ok()
        .and_then(|url| url.host_str().map(str::to_ascii_lowercase))
        .map(|host| host == "api.openai.com")
        .unwrap_or(false)
}

fn request_body(model: &str, data: &AiCompleteRequest, include_usage: bool) -> Value {
    let mut messages = Vec::new();
    if let Some(system) = data.system.as_deref() {
        messages.push(json!({ "role": "system", "content": system }));
    }
    messages.push(json!({ "role": "user", "content": data.prompt }));

    let mut body = json!({
        "model": model,
        "messages": messages,
        "stream": true,
    });

    if let Some(max_tokens) = data.max_tokens {
        body["max_tokens"] = json!(max_tokens);
    }
    if include_usage {
        body["stream_options"] = json!({ "include_usage": true });
    }

    body
}

pub async fn stream(
    client: &Client,
    app: &AppHandle,
    data: &AiCompleteRequest,
    model: &str,
    base_url: &str,
    api_key: &str,
    token: CancellationToken,
) -> Result<AiUsage, AppError> {
    let mut request = client
        .post(completions_url(base_url))
        .header("content-type", "application/json");

    if !api_key.is_empty() {
        request = request.header("authorization", format!("Bearer {api_key}"));
    }

    let response = request
        .json(&request_body(model, data, is_openai_api(base_url)))
        .send()
        .await
        .map_err(AppError::from)?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::new(format!("AI API error ({status}): {body}")));
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
        if let Some(model) = value["model"].as_str() {
            usage.model = model.to_string();
        }
        if let Some(text) = value["choices"][0]["delta"]["content"].as_str() {
            deltas.push(text.to_string());
        }
        if let Some(tokens) = value["usage"]["prompt_tokens"].as_u64() {
            usage.input_tokens = tokens as u32;
        }
        if let Some(tokens) = value["usage"]["completion_tokens"].as_u64() {
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
    fn request_body_uses_system_and_user_messages() {
        let mut data = request();
        data.system = Some("be brief".into());
        data.max_tokens = Some(64);

        let body = request_body("gpt-4.1", &data, true);
        assert_eq!(body["model"], "gpt-4.1");
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][0]["content"], "be brief");
        assert_eq!(body["messages"][1]["role"], "user");
        assert_eq!(body["messages"][1]["content"], "hello");
        assert_eq!(body["max_tokens"], 64);
        assert_eq!(body["stream_options"]["include_usage"], true);
    }

    #[test]
    fn endpoint_is_joined_once() {
        assert_eq!(
            completions_url("https://api.openai.com/v1"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            completions_url("https://api.openai.com/v1/chat/completions"),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn http_is_only_allowed_for_localhost() {
        assert!(resolve_base("http://localhost:11434/v1").is_ok());
        assert!(resolve_base("http://127.0.0.1:1234/v1/").is_ok());
        assert!(resolve_base("http://example.com/v1").is_err());
        assert!(resolve_base("https://openrouter.ai/api/v1").is_ok());
        assert!(resolve_base("ftp://example.com").is_err());
        assert!(resolve_base("https://example.com/v1?key=1").is_err());
    }

    #[test]
    fn stream_events_yield_text_and_usage() {
        let mut usage = AiUsage {
            model: "gpt-4.1".into(),
            ..Default::default()
        };

        let delta =
            r#"data: {"model":"gpt-4.1-2025-04-14","choices":[{"delta":{"content":"Hi"}}]}"#;
        let usage_event =
            r#"data: {"choices":[],"usage":{"prompt_tokens":9,"completion_tokens":2}}"#;

        assert_eq!(parse_event(delta, &mut usage).unwrap(), vec!["Hi"]);
        assert!(parse_event(usage_event, &mut usage).unwrap().is_empty());
        assert_eq!(usage.model, "gpt-4.1-2025-04-14");
        assert_eq!(usage.input_tokens, 9);
        assert_eq!(usage.output_tokens, 2);
    }

    #[test]
    fn error_events_become_errors() {
        let mut usage = AiUsage::default();
        let event = r#"data: {"error":{"message":"bad key"}}"#;
        assert_eq!(parse_event(event, &mut usage).unwrap_err().error, "bad key");
    }
}
