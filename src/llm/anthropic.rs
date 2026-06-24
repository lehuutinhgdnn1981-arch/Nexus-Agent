//! Anthropic provider — Claude Messages API.
//!
//! - Endpoint: `POST {base_url}/v1/messages`
//! - Headers: `x-api-key`, `anthropic-version: 2023-06-01`
//! - Streaming: SSE event types: `message_start`, `content_block_start`,
//!   `content_block_delta`, `content_block_stop`, `message_delta`, `message_stop`
//! - Tool calling: `tools` array với `input_schema`, tool calls là `content_block` type `tool_use`

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tracing::debug;

use crate::error::LlmError;
use crate::llm::provider::LLMProvider;
use crate::llm::streaming::SseParser;
use crate::llm::types::{
    ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, MessageRole, ToolCall,
    ToolCallFunction, Usage,
};

const PROVIDER_ID: &str = "anthropic";
const ANTHROPIC_VERSION: &str = "2023-06-01";

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
    embedding_model: String, // unused — Anthropic không có embeddings
}

impl AnthropicProvider {
    pub fn new(api_key: String, base_url: String, embedding_model: String) -> Result<Self, LlmError> {
        if api_key.is_empty() {
            return Err(LlmError::InvalidApiKey {
                provider: PROVIDER_ID.into(),
            });
        }
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(LlmError::Http)?;
        Ok(Self {
            client,
            api_key,
            base_url: base_url.trim_end_matches('/').to_string(),
            embedding_model,
        })
    }

    /// Convert ChatMessage[] → Anthropic format (system tách riêng, còn lại trong messages).
    /// Trả về (system_prompt, messages).
    fn convert_messages(messages: &[ChatMessage]) -> (String, Vec<Value>) {
        let mut system = String::new();
        let mut out: Vec<Value> = Vec::new();
        for m in messages {
            match m.role {
                MessageRole::System => {
                    if !system.is_empty() {
                        system.push('\n');
                    }
                    system.push_str(&m.content);
                }
                MessageRole::User => {
                    out.push(json!({"role": "user", "content": m.content}));
                }
                MessageRole::Assistant => {
                    if let Some(tool_calls) = &m.tool_calls {
                        let mut content = Vec::new();
                        if !m.content.is_empty() {
                            content.push(json!({"type": "text", "text": m.content}));
                        }
                        for tc in tool_calls {
                            let args: Value = serde_json::from_str(&tc.function.arguments)
                                .unwrap_or(Value::Null);
                            content.push(json!({
                                "type": "tool_use",
                                "id": tc.id,
                                "name": tc.function.name,
                                "input": args,
                            }));
                        }
                        out.push(json!({"role": "assistant", "content": content}));
                    } else {
                        out.push(json!({"role": "assistant", "content": m.content}));
                    }
                }
                MessageRole::Tool => {
                    out.push(json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": m.tool_call_id,
                            "content": m.content,
                        }],
                    }));
                }
            }
        }
        (system, out)
    }

    fn tools_to_json(req: &ChatRequest) -> Vec<Value> {
        req.tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.parameters,
                })
            })
            .collect()
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    fn id(&self) -> &'static str {
        PROVIDER_ID
    }

    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, LlmError> {
        let (system, messages) = Self::convert_messages(&req.messages);
        let mut body = json!({
            "model": req.model,
            "messages": messages,
            "max_tokens": req.max_tokens.unwrap_or(4096),
            "stream": false,
        });
        if !system.is_empty() {
            body["system"] = json!(system);
        }
        if !req.tools.is_empty() {
            body["tools"] = json!(Self::tools_to_json(&req));
        }
        if let Some(t) = req.temperature {
            body["temperature"] = json!(t);
        }

        let url = format!("{}/v1/messages", self.base_url);
        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::ProviderStatus {
                provider: PROVIDER_ID.into(),
                status: status.as_u16(),
                body,
            });
        }

        let v: Value = resp.json().await?;
        let content_arr = v
            .get("content")
            .and_then(|c| c.as_array())
            .ok_or_else(|| LlmError::MalformedResponse("missing content".into()))?;

        let mut text = String::new();
        let mut tool_calls = Vec::new();
        for block in content_arr {
            match block.get("type").and_then(|t| t.as_str()) {
                Some("text") => {
                    if let Some(t) = block.get("text").and_then(|t| t.as_str()) {
                        text.push_str(t);
                    }
                }
                Some("tool_use") => {
                    let id = block.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string();
                    let name = block
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();
                    let input = block.get("input").cloned().unwrap_or(Value::Null);
                    tool_calls.push(ToolCall {
                        id,
                        r#type: "function".into(),
                        function: ToolCallFunction {
                            name,
                            arguments: serde_json::to_string(&input).unwrap_or_default(),
                        },
                    });
                }
                _ => {}
            }
        }

        let usage = v
            .get("usage")
            .map(|u| Usage {
                prompt_tokens: u.get("input_tokens").and_then(|t| t.as_u64()).unwrap_or(0) as u32,
                completion_tokens: u
                    .get("output_tokens")
                    .and_then(|t| t.as_u64())
                    .unwrap_or(0) as u32,
                total_tokens: u
                    .get("input_tokens")
                    .and_then(|t| t.as_u64())
                    .unwrap_or(0) as u32
                    + u.get("output_tokens")
                        .and_then(|t| t.as_u64())
                        .unwrap_or(0) as u32,
            })
            .unwrap_or_default();

        Ok(ChatResponse {
            content: text,
            tool_calls,
            usage,
        })
    }

    async fn chat_stream(
        &self,
        req: ChatRequest,
        tx: mpsc::Sender<ChatStreamChunk>,
    ) -> Result<(), LlmError> {
        let (system, messages) = Self::convert_messages(&req.messages);
        let mut body = json!({
            "model": req.model,
            "messages": messages,
            "max_tokens": req.max_tokens.unwrap_or(4096),
            "stream": true,
        });
        if !system.is_empty() {
            body["system"] = json!(system);
        }
        if !req.tools.is_empty() {
            body["tools"] = json!(Self::tools_to_json(&req));
        }
        if let Some(t) = req.temperature {
            body["temperature"] = json!(t);
        }

        let url = format!("{}/v1/messages", self.base_url);
        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::ProviderStatus {
                provider: PROVIDER_ID.into(),
                status: status.as_u16(),
                body,
            });
        }

        let mut parser = SseParser::new();
        let mut stream = resp.bytes_stream();
        use futures::StreamExt;

        // Accumulate tool_use blocks: index → (id, name, args_string)
        let mut tool_acc: std::collections::BTreeMap<u32, (String, String, String)> =
            std::collections::BTreeMap::new();

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| LlmError::Stream(e.to_string()))?;
            let events = parser.feed(&bytes);
            for ev in events {
                let v: Value = match serde_json::from_str(&ev.data) {
                    Ok(v) => v,
                    Err(_) => {
                        debug!(data = %ev.data, "non-JSON SSE, skip");
                        continue;
                    }
                };
                let event_type = v
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("");

                match event_type {
                    "content_block_start" => {
                        if let Some(block) = v.get("content_block") {
                            if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                                let idx = v
                                    .get("index")
                                    .and_then(|i| i.as_u64())
                                    .unwrap_or(0) as u32;
                                let id = block
                                    .get("id")
                                    .and_then(|i| i.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let name = block
                                    .get("name")
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                tool_acc.insert(idx, (id, name, String::new()));
                            }
                        }
                    }
                    "content_block_delta" => {
                        if let Some(delta) = v.get("delta") {
                            match delta.get("type").and_then(|t| t.as_str()) {
                                Some("text_delta") => {
                                    if let Some(t) = delta.get("text").and_then(|t| t.as_str()) {
                                        let _ = tx
                                            .send(ChatStreamChunk::Delta(t.to_string()))
                                            .await;
                                    }
                                }
                                Some("input_json_delta") => {
                                    if let Some(part) =
                                        delta.get("partial_json").and_then(|p| p.as_str())
                                    {
                                        let idx = v
                                            .get("index")
                                            .and_then(|i| i.as_u64())
                                            .unwrap_or(0) as u32;
                                        if let Some(entry) = tool_acc.get_mut(&idx) {
                                            entry.2.push_str(part);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    "message_delta" => {
                        if let Some(u) = v.get("usage") {
                            let _ = tx
                                .send(ChatStreamChunk::Usage(Usage {
                                    prompt_tokens: u
                                        .get("input_tokens")
                                        .and_then(|t| t.as_u64())
                                        .unwrap_or(0) as u32,
                                    completion_tokens: u
                                        .get("output_tokens")
                                        .and_then(|t| t.as_u64())
                                        .unwrap_or(0) as u32,
                                    total_tokens: 0,
                                }))
                                .await;
                        }
                    }
                    "message_stop" => {
                        // emit accumulated tool calls
                        for (_, (id, name, args)) in tool_acc.clone().into_iter() {
                            let _ = tx
                                .send(ChatStreamChunk::ToolCall(ToolCall {
                                    id,
                                    r#type: "function".into(),
                                    function: ToolCallFunction { name, arguments: args },
                                }))
                                .await;
                        }
                        let _ = tx.send(ChatStreamChunk::Done).await;
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }

        // Fallback Done nếu không thấy message_stop
        for (_, (id, name, args)) in tool_acc.into_iter() {
            let _ = tx
                .send(ChatStreamChunk::ToolCall(ToolCall {
                    id,
                    r#type: "function".into(),
                    function: ToolCallFunction { name, arguments: args },
                }))
                .await;
        }
        let _ = tx.send(ChatStreamChunk::Done).await;
        Ok(())
    }

    async fn embed(&self, _text: &str) -> Result<Vec<f32>, LlmError> {
        // Anthropic không có embeddings API — caller phải config embedding provider khác
        Err(LlmError::Embedding(
            "Anthropic does not provide embeddings API. Configure OpenAI or Ollama for embeddings."
                .into(),
        ))
    }

    fn supports_tools(&self) -> bool {
        true
    }
}
