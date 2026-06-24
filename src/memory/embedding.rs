//! Embedding client — wraps LLMProvider::embed.

use std::sync::Arc;

use crate::error::{LlmError, Result};
use crate::llm::provider::LLMProvider;

/// Client sinh embedding cho text.
pub struct EmbeddingClient {
    provider: Arc<dyn LLMProvider>,
}

impl EmbeddingClient {
    #[must_use]
    pub fn new(provider: Arc<dyn LLMProvider>) -> Self {
        Self { provider }
    }

    /// Sinh embedding cho 1 đoạn text.
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        if text.trim().is_empty() {
            return Err(LlmError::Embedding("empty text".into()).into());
        }
        self.provider.embed(text).await.map_err(Into::into)
    }

    /// Sinh embeddings cho nhiều đoạn text (tuần tự — để tránh rate limit).
    pub async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut out = Vec::with_capacity(texts.len());
        for t in texts {
            out.push(self.embed(t).await?);
        }
        Ok(out)
    }
}
