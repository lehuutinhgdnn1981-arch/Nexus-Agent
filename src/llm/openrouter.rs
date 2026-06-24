//! OpenRouter provider — OpenAI-compatible API, chỉ khác base_url + headers.
//!
//! - Endpoint: `POST {base_url}/chat/completions`
//! - Streaming: SSE tương tự OpenAI
//! - Headers thêm `HTTP-Referer` + `X-Title` (optional, cho ranking)

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::sync::mpsc;

use crate::error::LlmError;
use crate::llm::openai::OpenAIProvider;
use crate::llm::provider::LLMProvider;
use crate::llm::types::{
    ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, MessageRole, ToolCall,
    ToolCallFunction, Usage,
};

const PROVIDER_ID: &str = "openrouter";

/// OpenRouter provider — wraps OpenAI-compatible logic nhưng custom headers.
pub struct OpenRouterProvider {
    inner: OpenAIProvider,
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenRouterProvider {
    pub fn new(api_key: String, base_url: String, embedding_model: String) -> Result<Self, LlmError> {
        if api_key.is_empty() {
            return Err(LlmError::InvalidApiKey {
                provider: PROVIDER_ID.into(),
            });
        }
        let inner = OpenAIProvider::new(api_key.clone(), base_url.clone(), embedding_model)?;
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(LlmError::Http)?;
        Ok(Self {
            inner,
            client,
            api_key,
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    fn message_to_json(msg: &ChatMessage) -> Value {
        // Same as OpenAI
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
impl LLMProvider for OpenRouterProvider {
    fn id(&self) -> &'static str {
        PROVIDER_ID
    }

    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, LlmError> {
        // Delegate to inner OpenAI provider (same wire format)
        self.inner.chat(req).await
    }

    async fn chat_stream(
        &self,
        req: ChatRequest,
        tx: mpsc::Sender<ChatStreamChunk>,
    ) -> Result<(), LlmError> {
        // OpenRouter hỗ trợ cùng format SSE như OpenAI — delegate luôn
        self.inner.chat_stream(req, tx).await
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>, LlmError> {
        // OpenRouter không có endpoint embeddings — fallback dùng OpenAI inner (nếu base_url config thành OpenAI).
        // Thực tế OpenRouter users nên config embedding provider riêng = OpenAI.
        self.inner.embed(text).await
    }

    fn supports_tools(&self) -> bool {
        true
    }
}
