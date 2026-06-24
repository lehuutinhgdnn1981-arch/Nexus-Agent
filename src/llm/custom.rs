//! Custom OpenAI-compatible provider.
//!
//! Cho phép user add bất kỳ LLM endpoint nào tuân theo OpenAI Chat Completions API:
//!   - Together AI
//!   - Groq
//!   - Mistral AI
//!   - Fireworks AI
//!   - Anyscale
//!   - vLLM (local)
//!   - LM Studio (local)
//!   - LiteLLM proxy
//!   - OpenRouter sub-routes (vd: anthropic/claude-3.5-sonnet)
//!   - DeepSeek, Qwen, Yi, ... bất kỳ vendor nào tương thích OpenAI
//!
//! Custom provider được config qua `[llm.custom.<id>]` section trong config.toml.
//! Mỗi custom provider có thể override:
//!   - api_key
//!   - base_url
//!   - default_model
//!   - extra_headers (vd: HTTP-Referer cho OpenRouter)
//!   - embedding_model (nếu provider có /embeddings endpoint)

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::debug;

use crate::error::LlmError;
use crate::llm::provider::LLMProvider;
use crate::llm::streaming::SseParser;
use crate::llm::types::{
    ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, MessageRole, ToolCall,
    ToolCallFunction, Usage,
};

/// Cấu hình cho 1 custom provider.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomProviderConfig {
    /// Display ID cho provider này (vd: "together", "groq", "vllm-local").
    pub id: String,

    /// API key. None nếu provider không cần auth (vd: LM Studio local).
    #[serde(default)]
    pub api_key: Option<String>,

    /// Base URL (vd: "https://api.together.xyz/v1").
    pub base_url: String,

    /// Default model khi không specify trong ChatRequest.
    #[serde(default)]
    pub default_model: Option<String>,

    /// Default model cho embeddings (nếu provider hỗ trợ).
    /// None = không hỗ trợ embed().
    #[serde(default)]
    pub embedding_model: Option<String>,

    /// Extra HTTP headers (vd: {"HTTP-Referer": "https://nexus.app"} cho OpenRouter).
    #[serde(default)]
    pub extra_headers: Option<HashMap<String, String>>,

    /// Display name cho logs/UI.
    #[serde(default)]
    pub display_name: Option<String>,

    /// Có support tool calling không? (một số local models chưa support)
    #[serde(default = "default_true")]
    pub supports_tools: bool,

    /// Request timeout seconds.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    120
}

impl CustomProviderConfig {
    /// Helper: tạo config cho Together AI.
    #[must_use]
    pub fn together(api_key: String) -> Self {
        Self {
            id: "together".into(),
            api_key: Some(api_key),
            base_url: "https://api.together.xyz/v1".into(),
            default_model: Some("meta-llama/Llama-3-70b-chat-hf".into()),
            embedding_model: None,
            extra_headers: None,
            display_name: Some("Together AI".into()),
            supports_tools: true,
            timeout_secs: 120,
        }
    }

    /// Helper: tạo config cho Groq.
    #[must_use]
    pub fn groq(api_key: String) -> Self {
        Self {
            id: "groq".into(),
            api_key: Some(api_key),
            base_url: "https://api.groq.com/openai/v1".into(),
            default_model: Some("llama-3.1-70b-versatile".into()),
            embedding_model: None,
            extra_headers: None,
            display_name: Some("Groq".into()),
            supports_tools: true,
            timeout_secs: 120,
        }
    }

    /// Helper: tạo config cho Mistral AI.
    #[must_use]
    pub fn mistral(api_key: String) -> Self {
        Self {
            id: "mistral".into(),
            api_key: Some(api_key),
            base_url: "https://api.mistral.ai/v1".into(),
            default_model: Some("mistral-large-latest".into()),
            embedding_model: Some("mistral-embed".into()),
            extra_headers: None,
            display_name: Some("Mistral AI".into()),
            supports_tools: true,
            timeout_secs: 120,
        }
    }

    /// Helper: tạo config cho DeepSeek.
    #[must_use]
    pub fn deepseek(api_key: String) -> Self {
        Self {
            id: "deepseek".into(),
            api_key: Some(api_key),
            base_url: "https://api.deepseek.com/v1".into(),
            default_model: Some("deepseek-chat".into()),
            embedding_model: None,
            extra_headers: None,
            display_name: Some("DeepSeek".into()),
            supports_tools: true,
            timeout_secs: 120,
        }
    }

    /// Helper: tạo config cho vLLM local.
    #[must_use]
    pub fn vllm(base_url: String, default_model: String) -> Self {
        Self {
            id: "vllm".into(),
            api_key: None,
            base_url,
            default_model: Some(default_model),
            embedding_model: None,
            extra_headers: None,
            display_name: Some("vLLM (local)".into()),
            supports_tools: false, // vLLM tool calling còn hạn chế
            timeout_secs: 300,     // local có thể chậm hơn
        }
    }

    /// Helper: tạo config cho LM Studio local.
    #[must_use]
    pub fn lm_studio() -> Self {
        Self {
            id: "lmstudio".into(),
            api_key: None,
            base_url: "http://localhost:1234/v1".into(),
            default_model: Some("local-model".into()),
            embedding_model: None,
            extra_headers: None,
            display_name: Some("LM Studio (local)".into()),
            supports_tools: true,
            timeout_secs: 300,
        }
    }

    /// Helper: tạo config cho LiteLLM proxy.
    #[must_use]
    pub fn litellm(base_url: String, api_key: Option<String>) -> Self {
        Self {
            id: "litellm".into(),
            api_key,
            base_url,
            default_model: Some("gpt-4o-mini".into()),
            embedding_model: None,
            extra_headers: None,
            display_name: Some("LiteLLM proxy".into()),
            supports_tools: true,
            timeout_secs: 120,
        }
    }

    /// Resolve API key: ưu tiên explicit, rồi env var `<ID_uppercase>_API_KEY`.
    #[must_use]
    pub fn resolved_api_key(&self) -> Option<String> {
        if let Some(k) = &self.api_key {
            if !k.is_empty() {
                return Some(k.clone());
            }
        }
        let env_var = format!("{}_API_KEY", self.id.to_uppercase().replace('-', "_"));
        std::env::var(&env_var).ok()
    }
}

/// Custom provider — wraps OpenAI Chat Completions API với configurable base_url + headers.
pub struct CustomProvider {
    cfg: CustomProviderConfig,
    client: Client,
}

impl CustomProvider {
    /// Tạo mới. Trả về lỗi nếu base_url rỗng.
    pub fn new(cfg: CustomProviderConfig) -> Result<Self, LlmError> {
        if cfg.base_url.trim().is_empty() {
            return Err(LlmError::NotConfigured(format!(
                "custom provider `{}` base_url is empty",
                cfg.id
            )));
        }

        let mut builder = Client::builder().timeout(std::time::Duration::from_secs(cfg.timeout_secs));
        if cfg.api_key.as_deref() == Some("") {
            // No-auth local provider — disable default Authorization header injection
        }
        let client = builder.build().map_err(LlmError::Http)?;

        Ok(Self { cfg, client })
    }

    fn auth_header(&self) -> Option<String> {
        self.cfg.resolved_api_key().map(|k| format!("Bearer {k}"))
    }

    fn message_to_json(msg: &ChatMessage) -> Value {
        let mut v = json!({
            "role": match msg.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "tool",
            },
            "content": msg.content,
        });
        if let Some(tc) = &msg.tool_calls {
            v["tool_calls"] = json!(tc.iter().map(|c| json!({
                "id": c.id,
                "type": c.r#type,
                "function": {
                    "name": c.function.name,
                    "arguments": c.function.arguments,
                }
            })).collect::<Vec<_>>());
        }
        if let Some(id) = &msg.tool_call_id {
            v["tool_call_id"] = json!(id);
        }
        if let Some(name) = &msg.name {
            v["name"] = json!(name);
        }
        v
    }

    fn tools_to_json(req: &ChatRequest) -> Vec<Value> {
        req.tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect()
    }
}

#[async_trait]
impl LLMProvider for CustomProvider {
    fn id(&self) -> &'static str {
        // Box leak để turn String thành 'static str (provider lifetime = app lifetime)
        // Alternative: đổi trait `id()` trả Cow<str> — nhưng break API cho 4 provider khác.
        // Box::leak chỉ gọi 1 lần per provider instance, không leak memory.
        let leaked: &'static str = Box::leak(self.cfg.id.clone().into_boxed_str());
        leaked
    }

    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, LlmError> {
        let model = if req.model.is_empty() {
            self.cfg
                .default_model
                .clone()
                .ok_or_else(|| LlmError::NotConfigured(format!("custom provider `{}` default_model missing", self.cfg.id)))?
        } else {
            req.model.clone()
        };

        let body = json!({
            "model": model,
            "messages": req.messages.iter().map(Self::message_to_json).collect::<Vec<_>>(),
            "tools": Self::tools_to_json(&req),
            "temperature": req.temperature.unwrap_or(0.7),
            "max_tokens": req.max_tokens,
            "stream": false,
        });

        let url = format!("{}/chat/completions", self.cfg.base_url.trim_end_matches('/'));
        let mut request = self.client.post(&url).json(&body);
        if let Some(auth) = self.auth_header() {
            request = request.header("Authorization", auth);
        }
        if let Some(headers) = &self.cfg.extra_headers {
            for (k, v) in headers {
                request = request.header(k, v);
            }
        }

        let resp = request.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::ProviderStatus {
                provider: self.cfg.id.clone(),
                status: status.as_u16(),
                body,
            });
        }

        let v: Value = resp.json().await?;
        let choice = v
            .get("choices")
            .and_then(|c| c.get(0))
            .ok_or_else(|| LlmError::MalformedResponse("missing choices[0]".into()))?;
        let msg = choice
            .get("message")
            .ok_or_else(|| LlmError::MalformedResponse("missing message".into()))?;

        let content = msg
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        let tool_calls: Vec<ToolCall> = msg
            .get("tool_calls")
            .and_then(|tc| tc.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| {
                        Some(ToolCall {
                            id: c.get("id")?.as_str()?.to_string(),
                            r#type: c
                                .get("type")
                                .and_then(|t| t.as_str())
                                .unwrap_or("function")
                                .to_string(),
                            function: ToolCallFunction {
                                name: c.get("function")?.get("name")?.as_str()?.to_string(),
                                arguments: c.get("function")?.get("arguments")?.as_str()?.to_string(),
                            },
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let usage = v
            .get("usage")
            .map(|u| Usage {
                prompt_tokens: u.get("prompt_tokens").and_then(|t| t.as_u64()).unwrap_or(0) as u32,
                completion_tokens: u
                    .get("completion_tokens")
                    .and_then(|t| t.as_u64())
                    .unwrap_or(0) as u32,
                total_tokens: u.get("total_tokens").and_then(|t| t.as_u64()).unwrap_or(0) as u32,
            })
            .unwrap_or_default();

        Ok(ChatResponse {
            content,
            tool_calls,
            usage,
        })
    }

    async fn chat_stream(
        &self,
        req: ChatRequest,
        tx: mpsc::Sender<ChatStreamChunk>,
    ) -> Result<(), LlmError> {
        let model = if req.model.is_empty() {
            self.cfg
                .default_model
                .clone()
                .ok_or_else(|| LlmError::NotConfigured(format!("custom provider `{}` default_model missing", self.cfg.id)))?
        } else {
            req.model.clone()
        };

        let body = json!({
            "model": model,
            "messages": req.messages.iter().map(Self::message_to_json).collect::<Vec<_>>(),
            "tools": Self::tools_to_json(&req),
            "temperature": req.temperature.unwrap_or(0.7),
            "max_tokens": req.max_tokens,
            "stream": true,
            "stream_options": { "include_usage": true },
        });

        let url = format!("{}/chat/completions", self.cfg.base_url.trim_end_matches('/'));
        let mut request = self.client.post(&url).json(&body);
        if let Some(auth) = self.auth_header() {
            request = request.header("Authorization", auth);
        }
        if let Some(headers) = &self.cfg.extra_headers {
            for (k, v) in headers {
                request = request.header(k, v);
            }
        }

        let resp = request.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::ProviderStatus {
                provider: self.cfg.id.clone(),
                status: status.as_u16(),
                body,
            });
        }

        let mut parser = SseParser::new();
        let mut stream = resp.bytes_stream();
        use futures::StreamExt;
        let mut final_usage: Option<Usage> = None;
        let mut tool_calls_acc: std::collections::BTreeMap<u32, (String, String, String)> =
            std::collections::BTreeMap::new();

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| LlmError::Stream(e.to_string()))?;
            let events = parser.feed(&bytes);
            for ev in events {
                if ev.data == "[DONE]" {
                    continue;
                }
                let v: Value = match serde_json::from_str(&ev.data) {
                    Ok(v) => v,
                    Err(_) => {
                        debug!(data = %ev.data, "non-JSON SSE data, skipping");
                        continue;
                    }
                };

                if let Some(u) = v.get("usage") {
                    final_usage = Some(Usage {
                        prompt_tokens: u
                            .get("prompt_tokens")
                            .and_then(|t| t.as_u64())
                            .unwrap_or(0) as u32,
                        completion_tokens: u
                            .get("completion_tokens")
                            .and_then(|t| t.as_u64())
                            .unwrap_or(0) as u32,
                        total_tokens: u
                            .get("total_tokens")
                            .and_then(|t| t.as_u64())
                            .unwrap_or(0) as u32,
                    });
                }

                let Some(choice) = v.get("choices").and_then(|c| c.get(0)) else {
                    continue;
                };
                let delta = choice.get("delta");

                if let Some(content) = delta.and_then(|d| d.get("content")).and_then(|c| c.as_str())
                {
                    if !content.is_empty() {
                        let _ = tx.send(ChatStreamChunk::Delta(content.to_string())).await;
                    }
                }

                if let Some(tcs) = delta.and_then(|d| d.get("tool_calls")).and_then(|t| t.as_array())
                {
                    for tc in tcs {
                        let idx = tc
                            .get("index")
                            .and_then(|i| i.as_u64())
                            .unwrap_or(0) as u32;
                        let entry = tool_calls_acc
                            .entry(idx)
                            .or_insert_with(|| (String::new(), String::new(), String::new()));
                        if let Some(id) = tc.get("id").and_then(|i| i.as_str()) {
                            entry.0 = id.to_string();
                        }
                        if let Some(name) = tc
                            .get("function")
                            .and_then(|f| f.get("name"))
                            .and_then(|n| n.as_str())
                        {
                            entry.1 = name.to_string();
                        }
                        if let Some(args) = tc
                            .get("function")
                            .and_then(|f| f.get("arguments"))
                            .and_then(|a| a.as_str())
                        {
                            entry.2.push_str(args);
                        }
                    }
                }
            }
        }

        for (_, (id, name, args)) in tool_calls_acc {
            let _ = tx
                .send(ChatStreamChunk::ToolCall(ToolCall {
                    id,
                    r#type: "function".into(),
                    function: ToolCallFunction { name, arguments: args },
                }))
                .await;
        }

        if let Some(u) = final_usage {
            let _ = tx.send(ChatStreamChunk::Usage(u)).await;
        }

        let _ = tx.send(ChatStreamChunk::Done).await;
        Ok(())
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>, LlmError> {
        let embedding_model = self
            .cfg
            .embedding_model
            .as_ref()
            .ok_or_else(|| LlmError::Embedding(format!("custom provider `{}` has no embedding_model", self.cfg.id)))?;

        let url = format!("{}/embeddings", self.cfg.base_url.trim_end_matches('/'));
        let body = json!({
            "model": embedding_model,
            "input": text,
        });

        let mut request = self.client.post(&url).json(&body);
        if let Some(auth) = self.auth_header() {
            request = request.header("Authorization", auth);
        }
        if let Some(headers) = &self.cfg.extra_headers {
            for (k, v) in headers {
                request = request.header(k, v);
            }
        }

        let resp = request.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::ProviderStatus {
                provider: self.cfg.id.clone(),
                status: status.as_u16(),
                body,
            });
        }

        let v: Value = resp.json().await?;
        let emb = v
            .get("data")
            .and_then(|d| d.get(0))
            .and_then(|x| x.get("embedding"))
            .and_then(|e| e.as_array())
            .ok_or_else(|| LlmError::Embedding("missing embedding".into()))?;

        Ok(emb.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect())
    }

    fn supports_tools(&self) -> bool {
        self.cfg.supports_tools
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_together_helper() {
        let cfg = CustomProviderConfig::together("test_key".into());
        assert_eq!(cfg.id, "together");
        assert_eq!(cfg.base_url, "https://api.together.xyz/v1");
        assert!(cfg.supports_tools);
    }

    #[test]
    fn config_groq_helper() {
        let cfg = CustomProviderConfig::groq("test_key".into());
        assert_eq!(cfg.id, "groq");
        assert_eq!(cfg.base_url, "https://api.groq.com/openai/v1");
        assert!(cfg.default_model.as_deref().unwrap().contains("llama"));
    }

    #[test]
    fn config_vllm_helper() {
        let cfg = CustomProviderConfig::vllm(
            "http://localhost:8000/v1".into(),
            "meta-llama/Llama-3-8B".into(),
        );
        assert_eq!(cfg.id, "vllm");
        assert!(!cfg.supports_tools);
        assert_eq!(cfg.timeout_secs, 300);
    }

    #[test]
    fn config_lm_studio_no_auth() {
        let cfg = CustomProviderConfig::lm_studio();
        assert!(cfg.api_key.is_none());
        assert_eq!(cfg.base_url, "http://localhost:1234/v1");
    }

    #[test]
    fn config_resolve_api_key_from_env() {
        std::env::set_var("GROQ_API_KEY", "env_key_123");
        let cfg = CustomProviderConfig {
            id: "groq".into(),
            api_key: None,
            base_url: "https://api.groq.com/openai/v1".into(),
            default_model: None,
            embedding_model: None,
            extra_headers: None,
            display_name: None,
            supports_tools: true,
            timeout_secs: 120,
        };
        assert_eq!(cfg.resolved_api_key(), Some("env_key_123".into()));
        std::env::remove_var("GROQ_API_KEY");
    }

    #[test]
    fn config_resolved_api_key_prefers_explicit() {
        std::env::set_var("TESTPROV_API_KEY", "env_key");
        let cfg = CustomProviderConfig {
            id: "testprov".into(),
            api_key: Some("explicit_key".into()),
            base_url: "https://api.test.com/v1".into(),
            default_model: None,
            embedding_model: None,
            extra_headers: None,
            display_name: None,
            supports_tools: true,
            timeout_secs: 120,
        };
        assert_eq!(cfg.resolved_api_key(), Some("explicit_key".into()));
        std::env::remove_var("TESTPROV_API_KEY");
    }

    #[test]
    fn provider_construction_fails_on_empty_base_url() {
        let cfg = CustomProviderConfig {
            id: "broken".into(),
            api_key: None,
            base_url: "".into(),
            default_model: None,
            embedding_model: None,
            extra_headers: None,
            display_name: None,
            supports_tools: true,
            timeout_secs: 120,
        };
        let result = CustomProvider::new(cfg);
        assert!(result.is_err());
    }

    #[test]
    fn provider_construction_succeeds_for_local() {
        let cfg = CustomProviderConfig::lm_studio();
        let provider = CustomProvider::new(cfg);
        assert!(provider.is_ok());
    }

    #[test]
    fn config_serializes_to_toml() {
        let cfg = CustomProviderConfig::together("key".into());
        let toml_str = toml::to_string(&cfg).unwrap();
        assert!(toml_str.contains("together"));
        assert!(toml_str.contains("api.together.xyz"));
    }
}
