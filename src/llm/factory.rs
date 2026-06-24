//! LLM factory — build provider từ config.

use std::sync::Arc;

use crate::config::provider_config::ProviderConfig;
use crate::config::AppConfig;
use crate::error::{LlmError, Result};
use crate::llm::custom::CustomProviderConfig;
use crate::llm::provider::LLMProvider;

/// Build provider theo tên + config.
///
/// `name`: "openai" | "openrouter" | "anthropic" | "ollama" | custom provider id
/// `cfg`: ProviderConfig (api_key, base_url, default_model)
/// `embedding_model`: model dùng cho `embed()` (chỉ áp dụng OpenAI/Ollama)
pub fn build_provider(
    name: &str,
    cfg: &ProviderConfig,
    embedding_model: &str,
) -> Result<Arc<dyn LLMProvider>> {
    let provider: Arc<dyn LLMProvider> = match name {
        "openai" => {
            let key = cfg
                .resolved_api_key("OPENAI_API_KEY")
                .ok_or(LlmError::NotConfigured("openai api_key".into()))?;
            let base = cfg
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.openai.com/v1".into());
            Arc::new(crate::llm::openai::OpenAIProvider::new(key, base, embedding_model.to_string())?)
        }
        "openrouter" => {
            let key = cfg
                .resolved_api_key("OPENROUTER_API_KEY")
                .ok_or(LlmError::NotConfigured("openrouter api_key".into()))?;
            let base = cfg
                .base_url
                .clone()
                .unwrap_or_else(|| "https://openrouter.ai/api/v1".into());
            Arc::new(crate::llm::openrouter::OpenRouterProvider::new(
                key,
                base,
                embedding_model.to_string(),
            )?)
        }
        "anthropic" => {
            let key = cfg
                .resolved_api_key("ANTHROPIC_API_KEY")
                .ok_or(LlmError::NotConfigured("anthropic api_key".into()))?;
            let base = cfg
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.anthropic.com".into());
            Arc::new(crate::llm::anthropic::AnthropicProvider::new(
                key,
                base,
                embedding_model.to_string(),
            )?)
        }
        "ollama" => {
            let base = cfg
                .base_url
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".into());
            Arc::new(crate::llm::ollama::OllamaProvider::new(base, embedding_model.to_string())?)
        }
        other => {
            return Err(crate::error::NexusError::InvalidArgument(format!(
                "unknown LLM provider: {other} (use build_provider_from_app_config for custom providers)"
            )));
        }
    };
    Ok(provider)
}

/// Build provider từ AppConfig — hỗ trợ cả 4 built-in providers + custom providers.
///
/// `name`: "openai" | "openrouter" | "anthropic" | "ollama" | custom provider id
/// `config`: AppConfig
/// `embedding_model_override`: nếu None, dùng default embedding model từ config
pub fn build_provider_from_app_config(
    name: &str,
    config: &AppConfig,
    embedding_model_override: Option<&str>,
) -> Result<Arc<dyn LLMProvider>> {
    // Check built-in providers first
    if let Some(builtin_cfg) = config.provider(name) {
        let embedding_model = embedding_model_override.unwrap_or(&config.memory.embedding_model);
        return build_provider(name, builtin_cfg, embedding_model);
    }

    // Check custom providers
    if let Some(custom_cfg) = config.custom_provider(name) {
        let provider = crate::llm::custom::CustomProvider::new(custom_cfg.clone())?;
        return Ok(Arc::new(provider));
    }

    Err(crate::error::NexusError::InvalidArgument(format!(
        "unknown LLM provider: `{name}`. Available: {}",
        config
            .all_provider_ids()
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )))
}

/// Helper: build provider từ `CustomProviderConfig` trực tiếp (cho tests / dynamic add).
pub fn build_custom_provider(cfg: CustomProviderConfig) -> Result<Arc<dyn LLMProvider>> {
    let provider = crate::llm::custom::CustomProvider::new(cfg)?;
    Ok(Arc::new(provider))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_custom_provider_succeeds() {
        let cfg = CustomProviderConfig::lm_studio();
        let result = build_custom_provider(cfg);
        assert!(result.is_ok());
    }

    #[test]
    fn build_provider_from_app_config_finds_custom() {
        let mut config = AppConfig::defaults();
        config.llm.custom.insert("vllm".into(), CustomProviderConfig::vllm(
            "http://localhost:8000/v1".into(),
            "test-model".into(),
        ));

        let result = build_provider_from_app_config("vllm", &config, None);
        assert!(result.is_ok());
    }

    #[test]
    fn build_provider_from_app_config_finds_builtin() {
        let mut config = AppConfig::defaults();
        config.llm.openai.api_key = Some("sk-test".into());

        let result = build_provider_from_app_config("openai", &config, None);
        assert!(result.is_ok());
    }

    #[test]
    fn build_provider_from_app_config_unknown_fails() {
        let config = AppConfig::defaults();
        let result = build_provider_from_app_config("nonexistent", &config, None);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Available"));
    }

    #[test]
    fn all_provider_ids_includes_custom() {
        let mut config = AppConfig::defaults();
        config.llm.custom.insert("together".into(), CustomProviderConfig::together("key".into()));
        config.llm.custom.insert("groq".into(), CustomProviderConfig::groq("key".into()));

        let ids = config.all_provider_ids();
        assert!(ids.contains(&"openai".to_string()));
        assert!(ids.contains(&"together".to_string()));
        assert!(ids.contains(&"groq".to_string()));
        assert_eq!(ids.len(), 6); // 4 built-in + 2 custom
    }
}
