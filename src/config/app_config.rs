//! App configuration (top-level config struct).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::provider_config::ProviderConfig;
use crate::llm::custom::CustomProviderConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub max_iterations: u32,
    pub max_tool_calls: u32,
    pub default_provider: String,
    pub default_model: String,
    pub system_prompt: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            max_tool_calls: 50,
            default_provider: "openai".into(),
            default_model: "gpt-4o-mini".into(),
            system_prompt: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub embedding_provider: String,         // 'openai' | 'ollama' | custom provider id
    pub embedding_model: String,
    pub embedding_dim: u32,
    pub recall_top_k: u32,
    pub dedup_threshold: f32,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            embedding_provider: "openai".into(),
            embedding_model: "text-embedding-3-small".into(),
            embedding_dim: 1536,
            recall_top_k: 5,
            dedup_threshold: 0.92,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub approval_timeout_secs: u64,
    pub shell_timeout_secs: u64,
    pub shell_max_output_kb: usize,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            approval_timeout_secs: 300,
            shell_timeout_secs: 60,
            shell_max_output_kb: 256,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    pub headless: bool,
    pub port: u16,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            headless: true,
            port: 9222,
        }
    }
}

/// LLM config — 4 built-in providers + map custom providers (OpenAI-compatible).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LlmConfig {
    pub openai: ProviderConfig,
    pub openrouter: ProviderConfig,
    pub anthropic: ProviderConfig,
    pub ollama: ProviderConfig,
    /// Custom OpenAI-compatible providers — keyed by ID (vd: "together", "groq", "vllm").
    /// Mỗi entry là `CustomProviderConfig` với base_url + api_key + default_model + extra_headers.
    #[serde(default)]
    pub custom: HashMap<String, CustomProviderConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchConfig {
    pub default: String,                    // 'duckduckgo' | 'brave'
    pub brave_api_key: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub agent: AgentConfig,
    pub llm: LlmConfig,
    pub memory: MemoryConfig,
    pub security: SecurityConfig,
    pub browser: BrowserConfig,
    pub search: SearchConfig,
}

impl AppConfig {
    /// Tạo config mặc định với API keys đọc từ env vars.
    pub fn defaults() -> Self {
        let mut llm = LlmConfig::default();
        llm.openai = ProviderConfig::openai_default();
        llm.openrouter = ProviderConfig::openrouter_default();
        llm.anthropic = ProviderConfig::anthropic_default();
        llm.ollama = ProviderConfig::ollama_default();

        // Auto-detect custom providers từ env vars
        if let Ok(key) = std::env::var("TOGETHER_API_KEY") {
            llm.custom.insert("together".into(), CustomProviderConfig::together(key));
        }
        if let Ok(key) = std::env::var("GROQ_API_KEY") {
            llm.custom.insert("groq".into(), CustomProviderConfig::groq(key));
        }
        if let Ok(key) = std::env::var("MISTRAL_API_KEY") {
            llm.custom.insert("mistral".into(), CustomProviderConfig::mistral(key));
        }
        if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
            llm.custom.insert("deepseek".into(), CustomProviderConfig::deepseek(key));
        }

        Self {
            agent: AgentConfig::default(),
            llm,
            memory: MemoryConfig::default(),
            security: SecurityConfig::default(),
            browser: BrowserConfig::default(),
            search: SearchConfig::default(),
        }
    }

    /// Lấy `ProviderConfig` theo tên built-in provider.
    #[must_use]
    pub fn provider(&self, name: &str) -> Option<&ProviderConfig> {
        match name {
            "openai" => Some(&self.llm.openai),
            "openrouter" => Some(&self.llm.openrouter),
            "anthropic" => Some(&self.llm.anthropic),
            "ollama" => Some(&self.llm.ollama),
            _ => None,
        }
    }

    /// Lấy `CustomProviderConfig` theo id (vd: "together", "groq").
    #[must_use]
    pub fn custom_provider(&self, id: &str) -> Option<&CustomProviderConfig> {
        self.llm.custom.get(id)
    }

    /// List tất cả provider IDs (built-in + custom).
    #[must_use]
    pub fn all_provider_ids(&self) -> Vec<String> {
        let mut ids = vec![
            "openai".to_string(),
            "openrouter".to_string(),
            "anthropic".to_string(),
            "ollama".to_string(),
        ];
        ids.extend(self.llm.custom.keys().cloned());
        ids
    }
}
