//! OpenAI provider (Chat Completions API).
//!
//! - Endpoint: `POST {base_url}/chat/completions`
//! - Streaming: SSE với `data: {...}` (delta) + `data: [DONE]`
//! - Tool calling: tools array + `tool_calls` trong delta
//! - Embeddings: `POST {base_url}/embeddings`

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::error::LlmError;
use crate::llm::provider::LLMProvider;
use crate::llm::streaming::SseParser;
use crate::llm::types::{
    ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, MessageRole, ToolCall,
    ToolCallFunction, Usage,
};

const PROVIDER_ID: &str = "openai";

pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    base_url: String,
    embedding_model: String,
}

impl OpenAIProvider {
    /// Tạo mới. Trả về lỗi nếu không có API key.
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

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
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
impl LLMProvider for OpenAIProvider {
    fn id(&self) -> &'static str {
        PROVIDER_ID
    }

    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, LlmError> {
        let body = json!({
            "model": req.model,
            "messages": req.messages.iter().map(Self::message_to_json).collect::<Vec<_>>(),
            "tools": Self::tools_to_json(&req),
            "temperature": req.temperature.unwrap_or(0.7),
            "max_tokens": req.max_tokens,
            "stream": false,
        });

        let url = format!("{}/chat/completions", self.base_url);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
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
        let choice = v.get("choices").and_then(|c| c.get(0)).ok_or_else(|| {
            LlmError::MalformedResponse("missing choices[0]".into())
        })?;
        let msg = choice.get("message").ok_or_else(|| {
            LlmError::MalformedResponse("missing message".into())
        })?;

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
                                name: c
                                    .get("function")?
                                    .get("name")?
                                    .as_str()?
                                    .to_string(),
                                arguments: c
                                    .get("function")?
                                    .get("arguments")?
                                    .as_str()?
                                    .to_string(),
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
        let body = json!({
            "model": req.model,
            "messages": req.messages.iter().map(Self::message_to_json).collect::<Vec<_>>(),
            "tools": Self::tools_to_json(&req),
            "temperature": req.temperature.unwrap_or(0.7),
            "max_tokens": req.max_tokens,
            "stream": true,
            "stream_options": { "include_usage": true },
        });

        let url = format!("{}/chat/completions", self.base_url);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
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
        let mut final_usage: Option<Usage> = None;
        // tool_calls accumulation theo index (vì OpenAI stream tool_calls theo index)
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

                // Usage ở cuối stream (choice = [])
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

                // Content delta
                if let Some(content) =
                    delta.and_then(|d| d.get("content")).and_then(|c| c.as_str())
                {
                    if !content.is_empty() {
                        let _ = tx.send(ChatStreamChunk::Delta(content.to_string())).await;
                    }
                }

                // Tool calls delta (accumulate theo index)
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

        // Flush partial events
        for ev in parser.flush() {
            warn!(?ev, "leftover SSE event (incomplete)");
        }

        // Emit accumulated tool calls
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
        let url = format!("{}/embeddings", self.base_url);
        let body = json!({
            "model": self.embedding_model,
            "input": text,
        });
        let resp = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
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
        let emb = v
            .get("data")
            .and_then(|d| d.get(0))
            .and_then(|x| x.get("embedding"))
            .and_then(|e| e.as_array())
            .ok_or_else(|| LlmError::Embedding("missing embedding".into()))?;

        Ok(emb.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect())
    }

    fn supports_tools(&self) -> bool {
        true
    }
}
