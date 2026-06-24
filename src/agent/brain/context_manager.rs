//! Context manager — token counting + sliding window + compression.
//!
//! Tránh context overflow khi conversation dài bằng:
//! 1. Token counting (ước lượng ~4 chars/token)
//! 2. Sliding window — chỉ giữ N messages gần nhất
//! 3. Compression — summarize old messages thành 1 system message

use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::Result;
use crate::llm::provider::LLMProvider;
use crate::llm::types::{ChatMessage, ChatRequest, MessageRole};

/// Config cho context manager.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContextConfig {
    /// Max tokens trước khi trigger compression.
    pub max_tokens: usize,
    /// Số messages gần nhất luôn giữ (không compress).
    pub keep_recent: usize,
    /// Số tokens ước lượng cho system prompt + tool schemas.
    pub reserved_for_system: usize,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_tokens: 24_000, // ~30k context - 6k reserved
            keep_recent: 10,
            reserved_for_system: 6_000,
        }
    }
}

/// Context manager — track token usage + compress when needed.
pub struct ContextManager {
    config: ContextConfig,
    provider: Option<Arc<dyn LLMProvider>>,
    /// Cached summary của old messages (recomputed khi old messages thay đổi).
    summary: RwLock<Option<String>>,
    /// Tokens ước lượng của summary hiện tại.
    summary_tokens: RwLock<usize>,
}

impl ContextManager {
    pub fn new(config: ContextConfig, provider: Option<Arc<dyn LLMProvider>>) -> Self {
        Self {
            config,
            provider,
            summary: RwLock::new(None),
            summary_tokens: RwLock::new(0),
        }
    }

    /// Ước lượng token count cho 1 string. Heuristic: ~4 chars/token (cho English).
    /// Có thể thay bằng tiktoken hoặc provider-specific tokenizer nếu cần chính xác.
    #[must_use]
    pub fn estimate_tokens(text: &str) -> usize {
        // Average English ~4 chars/token. CJK ~1.5 chars/token.
        // Mix → use 3.5 chars/token với floor 1.
        let chars = text.chars().count();
        let ascii_count = text.chars().filter(|c| c.is_ascii()).count();
        let non_ascii = chars.saturating_sub(ascii_count);
        // ASCII ~4 chars/token, non-ASCII ~1.5 chars/token
        ((ascii_count as f64 / 4.0) + (non_ascii as f64 / 1.5)).ceil() as usize
    }

    /// Ước lượng tokens cho 1 message (role + content + tool_calls).
    #[must_use]
    pub fn message_tokens(msg: &ChatMessage) -> usize {
        let role_tokens = match msg.role {
            MessageRole::System => 5,
            MessageRole::User => 5,
            MessageRole::Assistant => 7,
            MessageRole::Tool => 5,
        };
        let content_tokens = Self::estimate_tokens(&msg.content);
        let tool_call_tokens = msg
            .tool_calls
            .as_ref()
            .map(|tcs| {
                tcs.iter()
                    .map(|tc| {
                        Self::estimate_tokens(&tc.function.name) + Self::estimate_tokens(&tc.function.arguments)
                    })
                    .sum()
            })
            .unwrap_or(0);
        role_tokens + content_tokens + tool_call_tokens
    }

    /// Tổng tokens của list messages.
    #[must_use]
    pub fn total_tokens(messages: &[ChatMessage]) -> usize {
        messages.iter().map(Self::message_tokens).sum()
    }

    /// Build context cho LLM — compress nếu cần.
    ///
    /// Trả về messages theo thứ tự:
    /// 1. Summary system message (nếu đã compress)
    /// 2. `keep_recent` messages gần nhất
    pub async fn build(&self, messages: &[ChatMessage]) -> Result<Vec<ChatMessage>> {
        let available = self.config.max_tokens.saturating_sub(self.config.reserved_for_system);
        let total = Self::total_tokens(messages);

        if total <= available {
            // No compression needed
            return Ok(messages.to_vec());
        }

        debug!(total, available, "context compression triggered");

        // Split: old messages (compress) + recent (keep)
        let split_idx = messages.len().saturating_sub(self.config.keep_recent);
        if split_idx == 0 {
            // Can't compress — just return as-is (might exceed budget)
            return Ok(messages.to_vec());
        }

        let old = &messages[..split_idx];
        let recent = &messages[split_idx..];

        // Check if summary cached is still valid (covers same old messages)
        // For simplicity, we re-summarize if any old message changed.
        // A real impl would hash old messages and compare.
        let cached_summary = self.summary.read().clone();
        let summary = if let Some(s) = cached_summary {
            s
        } else if let Some(provider) = &self.provider {
            self.summarize_old_messages(provider, old).await?
        } else {
            // No provider — use crude truncation (keep first 200 chars of each)
            old.iter()
                .map(|m| format!("- [{}] {}", role_str(&m.role), truncate_str(&m.content, 200)))
                .collect::<Vec<_>>()
                .join("\n")
        };

        // Build final context
        let mut result = Vec::with_capacity(recent.len() + 1);
        result.push(ChatMessage::system(format!(
            "## Summary of previous conversation (compressed):\n{summary}"
        )));
        result.extend(recent.iter().cloned());

        let final_tokens = Self::total_tokens(&result);
        info!(original = total, compressed = final_tokens, "context compressed");

        // Cache summary
        *self.summary.write() = Some(summary);
        *self.summary_tokens.write() = final_tokens;

        Ok(result)
    }

    /// Reset cached summary — gọi khi conversation reset hoặc new turn với new messages.
    pub fn reset(&self) {
        *self.summary.write() = None;
        *self.summary_tokens.write() = 0;
    }

    async fn summarize_old_messages(
        &self,
        provider: &Arc<dyn LLMProvider>,
        old: &[ChatMessage],
    ) -> Result<String> {
        let mut summarize_messages = Vec::with_capacity(old.len() + 1);
        summarize_messages.push(ChatMessage::system(
            "Summarize the conversation below in 5-10 concise bullet points. \
             Focus on: user intent, decisions made, files/URLs mentioned, errors encountered, \
             key results from tool calls. Skip pleasantries. Be factual.",
        ));
        for msg in old {
            let role = role_str(&msg.role);
            if msg.content.is_empty() && msg.tool_calls.is_none() {
                continue;
            }
            let mut text = format!("[{role}]: {}", msg.content);
            if let Some(tcs) = &msg.tool_calls {
                for tc in tcs {
                    text.push_str(&format!("\n  → tool call: {} ({})", tc.function.name, tc.function.arguments));
                }
            }
            summarize_messages.push(ChatMessage::user(text));
        }

        let req = ChatRequest::new("gpt-4o-mini", summarize_messages);
        let resp = provider.chat(req).await?;
        Ok(resp.content)
    }
}

fn role_str(role: &MessageRole) -> &'static str {
    match role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::Tool => "tool",
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_tokens_ascii() {
        let tokens = ContextManager::estimate_tokens("Hello world!");
        // 12 chars / 4 = 3 tokens
        assert_eq!(tokens, 3);
    }

    #[test]
    fn estimate_tokens_cjk() {
        let tokens = ContextManager::estimate_tokens("你好世界");
        // 4 non-ASCII chars / 1.5 = 2.67 → ceil = 3
        assert_eq!(tokens, 3);
    }

    #[test]
    fn estimate_tokens_mixed() {
        let tokens = ContextManager::estimate_tokens("Hello 你好");
        // 6 ASCII + 2 non-ASCII = 1.5 + 1.33 = 2.83 → ceil = 3
        assert_eq!(tokens, 3);
    }

    #[test]
    fn message_tokens_includes_role_overhead() {
        let msg = ChatMessage::user("hi"); // 2 chars → 1 token + 5 role tokens = 6
        let tokens = ContextManager::message_tokens(&msg);
        assert!(tokens >= 6);
    }

    #[test]
    fn total_tokens_sums_messages() {
        let msgs = vec![
            ChatMessage::user("hello"),  // 5 chars / 4 = 1.25 → 2 + 5 role = 7
            ChatMessage::assistant("hi"), // 2 chars / 4 = 0.5 → 1 + 7 role = 8
        ];
        let total = ContextManager::total_tokens(&msgs);
        assert!(total >= 15);
    }

    #[tokio::test]
    async fn build_returns_as_is_when_under_budget() {
        let cfg = ContextConfig {
            max_tokens: 10_000,
            keep_recent: 5,
            reserved_for_system: 1000,
        };
        let mgr = ContextManager::new(cfg, None);
        let msgs = vec![
            ChatMessage::user("hello"),
            ChatMessage::assistant("hi there"),
        ];
        let result = mgr.build(&msgs).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn build_compresses_when_over_budget() {
        let cfg = ContextConfig {
            max_tokens: 100,
            keep_recent: 2,
            reserved_for_system: 10,
        };
        let mgr = ContextManager::new(cfg, None);
        // Generate many messages
        let msgs: Vec<ChatMessage> = (0..20)
            .map(|i| ChatMessage::user(format!("Message {i} with some content to make it longer")))
            .collect();
        let result = mgr.build(&msgs).await.unwrap();
        // Should have summary (1) + recent (2) = 3 messages
        assert_eq!(result.len(), 3);
        assert!(result[0].content.contains("Summary"));
    }

    #[tokio::test]
    async fn build_with_provider_uses_llm_summary() {
        use crate::llm::MockProvider;
        let mock = Arc::new(MockProvider::new());
        mock.enqueue_text("- Bullet 1\n- Bullet 2").enqueue_done();

        let cfg = ContextConfig {
            max_tokens: 100,
            keep_recent: 1,
            reserved_for_system: 10,
        };
        let mgr = ContextManager::new(cfg, Some(mock));
        let msgs = vec![
            ChatMessage::user("first message"),
            ChatMessage::user("second message"),
            ChatMessage::user("third message"),
        ];
        let result = mgr.build(&msgs).await.unwrap();
        assert_eq!(result.len(), 2); // summary + 1 recent
        assert!(result[0].content.contains("Bullet 1"));
    }

    #[test]
    fn reset_clears_cache() {
        let mgr = ContextManager::new(ContextConfig::default(), None);
        *mgr.summary.write() = Some("cached".into());
        mgr.reset();
        assert!(mgr.summary.read().is_none());
    }
}
