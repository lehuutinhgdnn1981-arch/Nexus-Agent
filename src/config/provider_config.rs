//! Provider configuration (OpenAI / OpenRouter / Anthropic / Ollama).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// API key. Nếu None, sẽ thử đọc từ env var `<PROVIDER>_API_KEY`.
    #[serde(default)]
    pub api_key: Option<String>,

    /// Base URL (override nếu cần, ví dụ cho OpenAI-compatible proxy).
    #[serde(default)]
    pub base_url: Option<String>,

    /// Default model cho provider này.
    #[serde(default)]
    pub default_model: Option<String>,
}

impl ProviderConfig {
    pub fn openai_default() -> Self {
        Self {
            api_key: std::env::var("OPENAI_API_KEY").ok(),
            base_url: Some("https://api.openai.com/v1".into()),
            default_model: Some("gpt-4o-mini".into()),
        }
    }

    pub fn openrouter_default() -> Self {
        Self {
            api_key: std::env::var("OPENROUTER_API_KEY").ok(),
            base_url: Some("https://openrouter.ai/api/v1".into()),
            default_model: Some("anthropic/claude-3.5-sonnet".into()),
        }
    }

    pub fn anthropic_default() -> Self {
        Self {
            api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
            base_url: Some("https://api.anthropic.com".into()),
            default_model: Some("claude-3-5-sonnet-20241022".into()),
        }
    }

    pub fn ollama_default() -> Self {
        Self {
            api_key: None,
            base_url: Some("http://localhost:11434".into()),
            default_model: Some("llama3.1".into()),
        }
    }

    /// Resolve API key: ưu tiên explicit, rồi env var.
    #[must_use]
    pub fn resolved_api_key(&self, env_var: &str) -> Option<String> {
        self.api_key
            .clone()
            .or_else(|| std::env::var(env_var).ok())
    }

    /// Lấy base_url hoặc trả về chuỗi rỗng.
    #[must_use]
    pub fn base_url_or_empty(&self) -> String {
        self.base_url.clone().unwrap_or_default()
    }
}
