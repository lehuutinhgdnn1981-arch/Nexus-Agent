//! Ollama provider — local LLM via HTTP API.
//!
//! - Endpoint: `POST {base_url}/api/chat` (chat), `POST {base_url}/api/embeddings` (embed)
//! - Streaming: NDJSON (1 JSON object per line, không phải SSE)
//! - Tool calling: `tools` array trong request body (chỉ vài model support: llama3.1+, qwen2.5+, mistral-nemo)

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tracing::debug;

use crate::error::LlmError;
use crate::llm::provider::LLMProvider;
use crate::llm::types::{
    ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, MessageRole, ToolCall,
    ToolCallFunction, Usage,
};

const PROVIDER_ID: &str = "ollama";

pub struct OllamaProvider {
    client: Client,
    base_url: String,
    embedding_model: String,
}

impl OllamaProvider {
    pub fn new(base_url: String, embedding_model: String) -> Result<Self, LlmError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // Ollama local có thể chậm
            .build()
            .map_err(LlmError::Http)?;
        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            embedding_model,
        })
    }

    fn message_to_json(msg: &ChatMessage) -> Value {
        let role = match msg.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        };
        let mut v = json!({
            "role": role,
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
impl LLMProvider for OllamaProvider {
    fn id(&self) -> &'static str {
        PROVIDER_ID
    }

    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, LlmError> {
        let mut body = json!({
            "model": req.model,
            "messages": req.messages.iter().map(Self::message_to_json).collect::<Vec<_>>(),
            "stream": false,
        });
        if !req.tools.is_empty() {
            body["tools"] = json!(Self::tools_to_json(&req));
        }
        if let Some(t) = req.temperature {
            body["options"] = json!({ "temperature": t });
        }

        let url = format!("{}/api/chat", self.base_url);
        let resp = self.client.post(&url).json(&body).send().await?;
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
        let msg = v
            .get("message")
            .ok_or_else(|| LlmError::MalformedResponse("missing message".into()))?;
        let content = msg
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        let tool_calls: Vec<ToolCall> = msg
            .get("tool_calls")
            .and_then(|t| t.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| {
                        Some(ToolCall {
                            id: c.get("id")?.as_str()?.to_string(),
                            r#type: "function".into(),
                            function: ToolCallFunction {
                                name: c
                                    .get("function")?
                                    .get("name")?
                                    .as_str()?
                                    .to_string(),
                                arguments: c
                                    .get("function")?
                                    .get("arguments")
                                    .and_then(|a| match a {
                                        Value::String(s) => Some(s.clone()),
                                        Value::Object(_) => Some(a.to_string()),
                                        _ => None,
                                    })
                                    .unwrap_or_default(),
                            },
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let usage = v
            .get("prompt_eval_count")
            .and_then(|_| {
                Some(Usage {
                    prompt_tokens: v
                        .get("prompt_eval_count")
                        .and_then(|t| t.as_u64())
                        .unwrap_or(0) as u32,
                    completion_tokens: v
                        .get("eval_count")
                        .and_then(|t| t.as_u64())
                        .unwrap_or(0) as u32,
                    total_tokens: 0,
                })
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
        let mut body = json!({
            "model": req.model,
            "messages": req.messages.iter().map(Self::message_to_json).collect::<Vec<_>>(),
            "stream": true,
        });
        if !req.tools.is_empty() {
            body["tools"] = json!(Self::tools_to_json(&req));
        }
        if let Some(t) = req.temperature {
            body["options"] = json!({ "temperature": t });
        }

        let url = format!("{}/api/chat", self.base_url);
        let resp = self.client.post(&url).json(&body).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::ProviderStatus {
                provider: PROVIDER_ID.into(),
                status: status.as_u16(),
                body,
            });
        }

        // Ollama stream = NDJSON: 1 JSON object per line
        let mut stream = resp.bytes_stream();
        use futures::StreamExt;
        let mut buffer = String::new();
        let mut final_tool_calls: Vec<ToolCall> = Vec::new();

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| LlmError::Stream(e.to_string()))?;
            buffer.push_str(std::str::from_utf8(&bytes).unwrap_or(""));
            while let Some(idx) = buffer.find('\n') {
                let line: String = buffer.drain(..idx + 1).collect();
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let v: Value = match serde_json::from_str(line) {
                    Ok(v) => v,
                    Err(_) => {
                        debug!(line, "non-JSON NDJSON line, skip");
                        continue;
                    }
                };
                if let Some(msg) = v.get("message") {
                    if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                        if !content.is_empty() {
                            let _ = tx.send(ChatStreamChunk::Delta(content.to_string())).await;
                        }
                    }
                    if let Some(tcs) = msg.get("tool_calls").and_then(|t| t.as_array()) {
                        for c in tcs {
                            if let Some(tc) = parse_tool_call(c) {
                                final_tool_calls.push(tc.clone());
                                let _ = tx.send(ChatStreamChunk::ToolCall(tc)).await;
                            }
                        }
                    }
                }
                if v.get("done").and_then(|d| d.as_bool()) == Some(true) {
                    if let (Some(prompt), Some(eval)) = (
                        v.get("prompt_eval_count").and_then(|x| x.as_u64()),
                        v.get("eval_count").and_then(|x| x.as_u64()),
                    ) {
                        let _ = tx
                            .send(ChatStreamChunk::Usage(Usage {
                                prompt_tokens: prompt as u32,
                                completion_tokens: eval as u32,
                                total_tokens: (prompt + eval) as u32,
                            }))
                            .await;
                    }
                    let _ = tx.send(ChatStreamChunk::Done).await;
                    return Ok(());
                }
            }
        }

        // Fallback Done
        let _ = tx.send(ChatStreamChunk::Done).await;
        Ok(())
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>, LlmError> {
        let url = format!("{}/api/embeddings", self.base_url);
        let body = json!({
            "model": self.embedding_model,
            "prompt": text,
        });
        let resp = self.client.post(&url).json(&body).send().await?;
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
            .get("embedding")
            .and_then(|e| e.as_array())
            .ok_or_else(|| LlmError::Embedding("missing embedding".into()))?;
        Ok(emb.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect())
    }

    fn supports_tools(&self) -> bool {
        true
    }
}

fn parse_tool_call(c: &Value) -> Option<ToolCall> {
    Some(ToolCall {
        id: c.get("id")?.as_str()?.to_string(),
        r#type: "function".into(),
        function: ToolCallFunction {
            name: c.get("function")?.get("name")?.as_str()?.to_string(),
            arguments: c
                .get("function")?
                .get("arguments")
                .and_then(|a| match a {
                    Value::String(s) => Some(s.clone()),
                    Value::Object(_) | Value::Array(_) => Some(a.to_string()),
                    _ => None,
                })
                .unwrap_or_default(),
        },
    })
}
