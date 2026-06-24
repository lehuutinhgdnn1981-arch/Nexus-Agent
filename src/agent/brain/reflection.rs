//! Reflection loop — agent tự critique sau tool call failure.
//!
//! Khi tool call thất bại, thay vì retry blind hoặc abort, agent:
//! 1. Phân tích root cause.
//! 2. Đề xuất alternative approach.
//! 3. Retry với input điều chỉnh (hoặc skip + continue).
//!
//! Inspired by Reflexion paper (https://arxiv.org/abs/2303.11366).

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::Result;
use crate::llm::provider::LLMProvider;
use crate::llm::types::{ChatMessage, ChatRequest};

/// Kết quả reflection.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReflectionResult {
    /// Nguyên nhân gốc rễ failure.
    pub root_cause: String,
    /// Cách tiếp cận thay thế.
    pub alternative_approach: String,
    /// Có nên retry với input mới không.
    pub should_retry: bool,
    /// Input mới cho retry (nếu should_retry = true).
    pub revised_input: Option<serde_json::Value>,
    /// Có nên skip tool này và continue không.
    pub should_skip: bool,
}

impl ReflectionResult {
    /// Default khi không reflect được — skip + continue.
    #[must_use]
    pub fn default_skip() -> Self {
        Self {
            root_cause: "unknown".into(),
            alternative_approach: "continue without this tool".into(),
            should_retry: false,
            revised_input: None,
            should_skip: true,
        }
    }
}

/// Reflect on tool call failure — gọi khi tool return error.
///
/// `tool_name`: tên tool fail.
/// `input`: input gốc.
/// `error`: error message từ tool.
/// `recent_history`: vài messages gần nhất cho context.
pub async fn reflect_on_failure(
    provider: &Arc<dyn LLMProvider>,
    model: &str,
    tool_name: &str,
    input: &serde_json::Value,
    error: &str,
    recent_history: &[ChatMessage],
) -> Result<ReflectionResult> {
    let system = format!(
        "You are a reflection agent. A tool call failed. Analyze the failure and recommend next steps.\n\n\
         Failed tool: `{tool_name}`\n\
         Input: `{input}`\n\
         Error: {error}\n\n\
         Respond with a JSON object (no markdown):\n\
         {{\n  \"root_cause\": \"<one-sentence cause>\",\n  \"alternative_approach\": \"<one-sentence suggestion>\",\n  \"should_retry\": <true|false>,\n  \"revised_input\": <JSON object or null>,\n  \"should_skip\": <true|false>\n}}\n\n\
         Rules:\n\
         - should_retry=true only if revised_input is meaningfully different and likely to fix the issue.\n\
         - should_skip=true if the task can continue without this tool succeeding.\n\
         - For permission denied / sandbox violation: should_skip=true, revised_input=null.\n\
         - For not-found errors: should_retry=false if path is fundamentally wrong, true if typo.\n\
         - For timeout: should_retry=true with shorter input or smaller scope."
    );

    let mut messages = vec![ChatMessage::system(system)];
    // Add recent history (last 5 messages, skip system)
    for msg in recent_history.iter().rev().take(5).rev() {
        if matches!(msg.role, crate::llm::types::MessageRole::System) {
            continue;
        }
        messages.push(msg.clone());
    }

    let req = ChatRequest::new(model, messages);
    let resp = provider.chat(req).await?;
    let result: ReflectionResult = parse_reflection_response(&resp.content)?;
    info!(
        tool = tool_name,
        root_cause = %result.root_cause,
        should_retry = result.should_retry,
        should_skip = result.should_skip,
        "reflection complete"
    );
    Ok(result)
}

/// Parse reflection response — tolerant với markdown fences.
fn parse_reflection_response(text: &str) -> Result<ReflectionResult> {
    let cleaned = text
        .trim()
        .strip_prefix("```json")
        .or_else(|| text.trim().strip_prefix("```"))
        .unwrap_or(text)
        .trim()
        .trim_end_matches("```")
        .trim();

    let result: ReflectionResult = serde_json::from_str(cleaned).map_err(|e| {
        warn!(error = %e, raw = %text, "failed to parse reflection JSON");
        crate::error::NexusError::Internal(format!("reflection parse error: {e}"))
    })?;

    // Sanity: should_retry=true requires revised_input
    if result.should_retry && result.revised_input.is_none() {
        warn!("reflection says should_retry=true but no revised_input — defaulting to skip");
        return Ok(ReflectionResult::default_skip());
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MockProvider;

    #[test]
    fn default_skip_is_safe() {
        let r = ReflectionResult::default_skip();
        assert!(r.should_skip);
        assert!(!r.should_retry);
        assert!(r.revised_input.is_none());
    }

    #[tokio::test]
    async fn reflect_returns_parsed_result() {
        let mock = Arc::new(MockProvider::new());
        mock.enqueue_text(r#"{"root_cause":"typo in path","alternative_approach":"fix the filename","should_retry":true,"revised_input":{"path":"correct.txt"},"should_skip":false}"#).enqueue_done();

        let result = reflect_on_failure(
            &mock,
            "mock-model",
            "read_file",
            &serde_json::json!({"path": "wrong.txt"}),
            "file not found",
            &[],
        )
        .await
        .unwrap();

        assert_eq!(result.root_cause, "typo in path");
        assert!(result.should_retry);
        assert!(result.revised_input.is_some());
        assert_eq!(result.revised_input.unwrap()["path"], "correct.txt");
    }

    #[tokio::test]
    async fn reflect_with_permission_denied_returns_skip() {
        let mock = Arc::new(MockProvider::new());
        mock.enqueue_text(r#"{"root_cause":"sandbox violation","alternative_approach":"use workspace path","should_retry":false,"revised_input":null,"should_skip":true}"#).enqueue_done();

        let result = reflect_on_failure(
            &mock,
            "mock-model",
            "write_file",
            &serde_json::json!({"path": "/etc/passwd"}),
            "sandbox violation",
            &[],
        )
        .await
        .unwrap();

        assert!(result.should_skip);
        assert!(!result.should_retry);
    }

    #[test]
    fn parse_handles_markdown_fences() {
        let json = r#"```json
{"root_cause":"x","alternative_approach":"y","should_retry":false,"revised_input":null,"should_skip":true}
```"#;
        let result = parse_reflection_response(json).unwrap();
        assert_eq!(result.root_cause, "x");
    }

    #[test]
    fn parse_rejects_retry_without_revised_input() {
        let json = r#"{"root_cause":"x","alternative_approach":"y","should_retry":true,"revised_input":null,"should_skip":false}"#;
        let result = parse_reflection_response(json).unwrap();
        // Should default to skip
        assert!(result.should_skip);
    }

    #[test]
    fn parse_rejects_invalid_json() {
        let json = "not valid json";
        let result = parse_reflection_response(json);
        assert!(result.is_err());
    }
}
