use crate::error::AppError;

pub const DEFAULT_OPENAI_BASE: &str = "https://api.openai.com/v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiProvider {
    Anthropic,
    OpenAi,
    Google,
}

impl AiProvider {
    pub fn parse(raw: &str) -> Result<Self, AppError> {
        match raw.trim() {
            "anthropic" => Ok(Self::Anthropic),
            "openai" => Ok(Self::OpenAi),
            "google" => Ok(Self::Google),
            other => Err(AppError::new(format!(
                "Unknown AI provider '{other}'. Choose one on the AI screen."
            ))),
        }
    }

    pub fn default_model(self) -> &'static str {
        match self {
            Self::Anthropic => "claude-opus-5-5",
            Self::OpenAi => "gpt-4.1",
            Self::Google => "gemini-2.5-flash",
        }
    }

    pub fn secret_key(self) -> &'static str {
        match self {
            Self::Anthropic => "apikey_anthropic",
            Self::OpenAi => "apikey_openai",
            Self::Google => "apikey_google",
        }
    }

    pub fn key_required(self, base_url: Option<&str>) -> bool {
        match self {
            Self::OpenAi => base_url.map(|base| !is_local_host(base)).unwrap_or(true),
            Self::Anthropic | Self::Google => true,
        }
    }

    pub fn missing_key_message(self) -> &'static str {
        match self {
            Self::Anthropic => "No Anthropic API key saved. Add one on the AI screen.",
            Self::OpenAi => {
                "No OpenAI API key saved. Add one on the AI screen, or point the base URL at a local server."
            }
            Self::Google => "No Gemini API key saved. Add one on the AI screen.",
        }
    }
}

pub fn is_local_host(base_url: &str) -> bool {
    reqwest::Url::parse(base_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_ascii_lowercase))
        .map(|host| matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1" | "[::1]"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn providers_parse_from_their_setting_value() {
        assert_eq!(
            AiProvider::parse("anthropic").unwrap(),
            AiProvider::Anthropic
        );
        assert_eq!(AiProvider::parse("openai").unwrap(), AiProvider::OpenAi);
        assert_eq!(AiProvider::parse("google").unwrap(), AiProvider::Google);
        assert!(AiProvider::parse("copilot").is_err());
    }

    #[test]
    fn local_bases_do_not_require_a_key() {
        assert!(!AiProvider::OpenAi.key_required(Some("http://localhost:11434/v1")));
        assert!(!AiProvider::OpenAi.key_required(Some("http://127.0.0.1:1234/v1")));
        assert!(AiProvider::OpenAi.key_required(Some("https://api.openai.com/v1")));
        assert!(AiProvider::Anthropic.key_required(None));
    }
}
