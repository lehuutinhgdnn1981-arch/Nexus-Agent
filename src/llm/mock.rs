//! Mock LLM provider — dùng cho agent loop integration tests.
//!
//! Provider này trả về scripted responses:
//! - `enqueue_text(str)` — queue text chunks để stream ra
//! - `enqueue_tool_call(name, args)` — queue tool call
//! - `enqueue_done()` — queue Done marker
//! - `set_embedding(vec)` — set fixed embedding trả về cho `embed()`

#![cfg(any(test, feature = "test-utils"))]

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::error::LlmError;
use crate::llm::provider::LLMProvider;
use crate::llm::types::{ChatRequest, ChatResponse, ChatStreamChunk, ToolCall, ToolCallFunction, Usage};

const PROVIDER_ID: &str = "mock";

#[derive(Debug, Clone)]
enum ScriptedChunk {
    Text(String),
    ToolCall(ToolCall),
    Done,
}

/// Mock LLM provider — trả về scripted chunks.
#[derive(Default)]
pub struct MockProvider {
    chunks: Arc<Mutex<Vec<ScriptedChunk>>>,
    fixed_embedding: Arc<Mutex<Option<Vec<f32>>>>,
    chat_call_count: Arc<Mutex<usize>>,
    stream_call_count: Arc<Mutex<usize>>,
    embed_call_count: Arc<Mutex<usize>>,
    supports_tools: bool,
}

impl MockProvider {
    #[must_use]
    pub fn new() -> Self {
        Self {
            chunks: Arc::new(Mutex::new(Vec::new())),
            fixed_embedding: Arc::new(Mutex::new(None)),
            chat_call_count: Arc::new(Mutex::new(0)),
            stream_call_count: Arc::new(Mutex::new(0)),
            embed_call_count: Arc::new(Mutex::new(0)),
            supports_tools: true,
        }
    }

    /// Tạo mới với fixed embedding (cho memory tests).
    #[must_use]
    pub fn with_embedding(emb: Vec<f32>) -> Self {
        let p = Self::new();
        *p.fixed_embedding.lock().unwrap() = Some(emb);
        p
    }

    /// Disable tool calling support.
    #[must_use]
    pub fn without_tools(mut self) -> Self {
        self.supports_tools = false;
        self
    }

    /// Enqueue text chunk to be streamed.
    pub fn enqueue_text(&self, text: impl Into<String>) -> &Self {
        self.chunks
            .lock()
            .unwrap()
            .push(ScriptedChunk::Text(text.into()));
        self
    }

    /// Enqueue a tool call.
    pub fn enqueue_tool_call(&self, id: impl Into<String>, name: impl Into<String>, args: serde_json::Value) -> &Self {
        self.chunks.lock().unwrap().push(ScriptedChunk::ToolCall(ToolCall {
            id: id.into(),
            r#type: "function".into(),
            function: ToolCallFunction {
                name: name.into(),
                arguments: serde_json::to_string(&args).unwrap_or_default(),
            },
        }));
        self
    }

    /// Enqueue a Done marker.
    pub fn enqueue_done(&self) -> &Self {
        self.chunks.lock().unwrap().push(ScriptedChunk::Done);
        self
    }

    /// Set fixed embedding to return for `embed()`.
    pub fn set_embedding(&self, emb: Vec<f32>) {
        *self.fixed_embedding.lock().unwrap() = Some(emb);
    }

    /// Get number of `chat()` calls.
    #[must_use]
    pub fn chat_call_count(&self) -> usize {
        *self.chat_call_count.lock().unwrap()
    }

    /// Get number of `chat_stream()` calls.
    #[must_use]
    pub fn stream_call_count(&self) -> usize {
        *self.stream_call_count.lock().unwrap()
    }

    /// Get number of `embed()` calls.
    #[must_use]
    pub fn embed_call_count(&self) -> usize {
        *self.embed_call_count.lock().unwrap()
    }

    fn drain_chunks(&self) -> Vec<ScriptedChunk> {
        std::mem::take(&mut *self.chunks.lock().unwrap())
    }
}

#[async_trait]
impl LLMProvider for MockProvider {
    fn id(&self) -> &'static str {
        PROVIDER_ID
    }

    async fn chat(&self, _req: ChatRequest) -> Result<ChatResponse, LlmError> {
        *self.chat_call_count.lock().unwrap() += 1;
        let chunks = self.drain_chunks();
        let mut content = String::new();
        let mut tool_calls = Vec::new();
        for chunk in chunks {
            match chunk {
                ScriptedChunk::Text(t) => content.push_str(&t),
                ScriptedChunk::ToolCall(tc) => tool_calls.push(tc),
                ScriptedChunk::Done => {}
            }
        }
        Ok(ChatResponse {
            content,
            tool_calls,
            usage: Usage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            },
        })
    }

    async fn chat_stream(
        &self,
        _req: ChatRequest,
        tx: mpsc::Sender<ChatStreamChunk>,
    ) -> Result<(), LlmError> {
        *self.stream_call_count.lock().unwrap() += 1;
        let chunks = self.drain_chunks();
        for chunk in chunks {
            match chunk {
                ScriptedChunk::Text(t) => {
                    let _ = tx.send(ChatStreamChunk::Delta(t)).await;
                }
                ScriptedChunk::ToolCall(tc) => {
                    let _ = tx.send(ChatStreamChunk::ToolCall(tc)).await;
                }
                ScriptedChunk::Done => {
                    let _ = tx.send(ChatStreamChunk::Done).await;
                }
            }
        }
        // Always emit Done if not in chunks
        let _ = tx.send(ChatStreamChunk::Done).await;
        Ok(())
    }

    async fn embed(&self, _text: &str) -> Result<Vec<f32>, LlmError> {
        *self.embed_call_count.lock().unwrap() += 1;
        self.fixed_embedding
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| LlmError::Embedding("no embedding set".into()))
    }

    fn supports_tools(&self) -> bool {
        self.supports_tools
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::types::{ChatMessage, MessageRole};

    #[tokio::test]
    async fn mock_chat_returns_scripted_content() {
        let p = MockProvider::new();
        p.enqueue_text("Hello").enqueue_text(" world").enqueue_done();

        let req = ChatRequest::new("mock-model", vec![ChatMessage::user("hi")]);
        let resp = p.chat(req).await.unwrap();
        assert_eq!(resp.content, "Hello world");
        assert!(resp.tool_calls.is_empty());
        assert_eq!(p.chat_call_count(), 1);
    }

    #[tokio::test]
    async fn mock_chat_stream_emits_chunks() {
        let p = MockProvider::new();
        p.enqueue_text("A").enqueue_text("B").enqueue_done();

        let (tx, mut rx) = mpsc::channel(64);
        let req = ChatRequest::new("mock-model", vec![ChatMessage::user("hi")]);
        p.chat_stream(req, tx).await.unwrap();

        let mut deltas = Vec::new();
        let mut dones = 0;
        while let Some(chunk) = rx.recv().await {
            match chunk {
                ChatStreamChunk::Delta(t) => deltas.push(t),
                ChatStreamChunk::Done => dones += 1,
                _ => {}
            }
        }
        assert_eq!(deltas, vec!["A".to_string(), "B".to_string()]);
        assert_eq!(dones, 1);
    }

    #[tokio::test]
    async fn mock_embed_returns_fixed() {
        let p = MockProvider::with_embedding(vec![0.1, 0.2, 0.3]);
        let emb = p.embed("hello").await.unwrap();
        assert_eq!(emb, vec![0.1, 0.2, 0.3]);
        assert_eq!(p.embed_call_count(), 1);
    }

    #[tokio::test]
    async fn mock_tool_call_enqueued() {
        let p = MockProvider::new();
        p.enqueue_tool_call("tc1", "read_file", serde_json::json!({"path": "test.txt"}))
            .enqueue_done();

        let req = ChatRequest::new("mock-model", vec![ChatMessage::user("hi")]);
        let resp = p.chat(req).await.unwrap();
        assert_eq!(resp.tool_calls.len(), 1);
        assert_eq!(resp.tool_calls[0].function.name, "read_file");
    }

    #[test]
    fn mock_supports_tools_default_true() {
        let p = MockProvider::new();
        assert!(p.supports_tools());
    }

    #[test]
    fn mock_without_tools_disables() {
        let p = MockProvider::new().without_tools();
        assert!(!p.supports_tools());
    }
}
