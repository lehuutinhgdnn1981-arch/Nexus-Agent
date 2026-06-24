//! `LLMProvider` trait — abstraction cho 4 provider.

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::error::LlmError;
use crate::llm::types::{ChatRequest, ChatResponse, ChatStreamChunk};

/// Provider abstraction. Mọi provider implement trait này và agent
/// dùng `Arc<dyn LLMProvider>` để gọi.
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// ID ngắn của provider (vd: "openai", "anthropic").
    fn id(&self) -> &'static str;

    /// Non-streaming chat. Trả về full response.
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, LlmError>;

    /// Streaming chat. Gửi chunks qua `tx`, trả về khi stream kết thúc.
    /// `tx` có buffer 64 chunk.
    async fn chat_stream(
        &self,
        req: ChatRequest,
        tx: mpsc::Sender<ChatStreamChunk>,
    ) -> Result<(), LlmError>;

    /// Sinh embedding cho 1 đoạn text (dùng cho memory system).
    async fn embed(&self, text: &str) -> Result<Vec<f32>, LlmError>;

    /// Provider có hỗ trợ tool calling không.
    fn supports_tools(&self) -> bool;
}
