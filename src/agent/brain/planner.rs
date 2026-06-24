//! Plan-and-execute pattern.
//!
//! Thay vì ReAct basic (1-shot reactive), plan-and-execute:
//! 1. Sinh full plan trước (list of steps).
//! 2. Execute từng step với mini-ReAct loop.
//! 3. Sau mỗi step, có thể revise plan.
//!
//! Heuristic trigger: bật plan-execute khi user message phức tạp:
//! - Chứa "and then", "after that", "step by step", "first... then..."
//! - Dài >100 chars
//! - Yêu cầu nhiều tool categories (file + shell + browser)

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::Result;
use crate::llm::provider::LLMProvider;
use crate::llm::types::{ChatMessage, ChatRequest};

/// Một step trong plan.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlanStep {
    /// Mô tả step (vd: "Read config.toml to get API key").
    pub description: String,
    /// Tools dự kiến dùng (vd: ["read_file"]).
    pub expected_tools: Vec<String>,
    /// Status hiện tại.
    pub status: PlanStepStatus,
    /// Result sau khi step hoàn thành.
    pub result: Option<String>,
}

/// Status của 1 plan step.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStepStatus {
    /// Chưa bắt đầu.
    Pending,
    /// Đang thực thi.
    InProgress,
    /// Hoàn thành thành công.
    Done,
    /// Thất bại.
    Failed,
    /// Bỏ qua (sau khi revise plan).
    Skipped,
}

impl PlanStepStatus {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

/// Full plan.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentPlan {
    /// Mục tiêu tổng quát.
    pub goal: String,
    /// Các steps theo thứ tự.
    pub steps: Vec<PlanStep>,
    /// Index của step đang execute.
    pub current_step: usize,
}

impl AgentPlan {
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.steps.iter().all(|s| {
            s.status == PlanStepStatus::Done || s.status == PlanStepStatus::Skipped
        })
    }

    #[must_use]
    pub fn current_step(&self) -> Option<&PlanStep> {
        self.steps.get(self.current_step)
    }

    pub fn advance(&mut self) {
        self.current_step = self.current_step.saturating_add(1);
    }
}

/// Heuristic: quyết định có bật plan-execute không.
#[must_use]
pub fn should_plan(user_message: &str) -> bool {
    let lower = user_message.to_lowercase();
    // Trigger patterns
    let trigger_phrases = [
        "and then",
        "after that",
        "step by step",
        "first ",
        "second ",
        "third ",
        "next ",
        "finally ",
        "subsequently",
        "thereafter",
        "followed by",
    ];
    for phrase in &trigger_phrases {
        if lower.contains(phrase) {
            return true;
        }
    }
    // Long message
    if user_message.chars().count() > 200 {
        return true;
    }
    // Multiple sentences
    let sentence_count = user_message.matches('.').count() + user_message.matches('!').count() + user_message.matches('?').count();
    if sentence_count >= 3 {
        return true;
    }
    false
}

/// Sinh plan từ user message + available tools.
pub async fn generate_plan(
    provider: &Arc<dyn LLMProvider>,
    model: &str,
    user_message: &str,
    available_tools: &[&str],
) -> Result<AgentPlan> {
    let tools_str = available_tools.join(", ");
    let system = format!(
        "You are a planning agent. Given a user request and available tools, create an execution plan.\n\n\
         Available tools: {tools_str}\n\n\
         Respond with a JSON object (no markdown, no explanation):\n\
         {{\n  \"goal\": \"<one-sentence goal>\",\n  \"steps\": [\n    {{\n      \"description\": \"<step description>\",\n      \"expected_tools\": [\"<tool_name>\", ...],\n      \"status\": \"pending\",\n      \"result\": null\n    }}\n  ]\n}}\n\n\
         Rules:\n\
         - 3-7 steps maximum.\n\
         - Each step should be specific and actionable.\n\
         - expected_tools must be from the available list.\n\
         - Skip steps that don't need tools (just describe them)."
    );

    let req = ChatRequest::new(
        model,
        vec![
            ChatMessage::system(system),
            ChatMessage::user(user_message.to_string()),
        ],
    );

    let resp = provider.chat(req).await?;
    let plan: AgentPlan = parse_plan_response(&resp.content)?;
    info!(goal = %plan.goal, steps = plan.steps.len(), "plan generated");
    Ok(plan)
}

/// Parse plan từ LLM response — tolerant với markdown fences.
fn parse_plan_response(text: &str) -> Result<AgentPlan> {
    let cleaned = text
        .trim()
        .strip_prefix("```json")
        .or_else(|| text.trim().strip_prefix("```"))
        .unwrap_or(text)
        .trim()
        .trim_end_matches("```")
        .trim();

    let plan: AgentPlan = serde_json::from_str(cleaned).map_err(|e| {
        warn!(error = %e, raw = %text, "failed to parse plan JSON");
        crate::error::NexusError::Internal(format!("plan parse error: {e}"))
    })?;

    if plan.steps.is_empty() {
        return Err(crate::error::NexusError::Internal(
            "plan has no steps".into(),
        ));
    }

    Ok(plan)
}

/// Format plan thành human-readable string (cho logging + UI).
#[must_use]
pub fn format_plan(plan: &AgentPlan) -> String {
    let mut out = format!("## Plan: {}\n\n", plan.goal);
    for (i, step) in plan.steps.iter().enumerate() {
        let marker = match step.status {
            PlanStepStatus::Pending => "○",
            PlanStepStatus::InProgress => "◐",
            PlanStepStatus::Done => "✓",
            PlanStepStatus::Failed => "✗",
            PlanStepStatus::Skipped => "–",
        };
        out.push_str(&format!("{marker} {}. {}\n", i + 1, step.description));
        if !step.expected_tools.is_empty() {
            out.push_str(&format!("   tools: {}\n", step.expected_tools.join(", ")));
        }
        if let Some(result) = &step.result {
            out.push_str(&format!("   result: {}\n", truncate(result, 100)));
        }
    }
    out
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MockProvider;

    #[test]
    fn should_plan_triggers_on_phrases() {
        assert!(should_plan("Read the file and then write a summary"));
        assert!(should_plan("First do X, second do Y, third do Z"));
        assert!(should_plan("Step by step, build the project"));
        assert!(should_plan("After that, run tests"));
    }

    #[test]
    fn should_plan_triggers_on_length() {
        let long = "a".repeat(250);
        assert!(should_plan(&long));
    }

    #[test]
    fn should_plan_triggers_on_multiple_sentences() {
        assert!(should_plan("Do X. Then do Y. Finally do Z."));
    }

    #[test]
    fn should_plan_skips_simple() {
        assert!(!should_plan("hello"));
        assert!(!should_plan("what is 2+2?"));
        assert!(!should_plan("read file.txt"));
    }

    #[test]
    fn plan_complete_check() {
        let plan = AgentPlan {
            goal: "test".into(),
            steps: vec![
                PlanStep {
                    description: "s1".into(),
                    expected_tools: vec![],
                    status: PlanStepStatus::Done,
                    result: Some("ok".into()),
                },
                PlanStep {
                    description: "s2".into(),
                    expected_tools: vec![],
                    status: PlanStepStatus::Skipped,
                    result: None,
                },
            ],
            current_step: 2,
        };
        assert!(plan.is_complete());
    }

    #[test]
    fn plan_not_complete_with_pending() {
        let plan = AgentPlan {
            goal: "test".into(),
            steps: vec![PlanStep {
                description: "s1".into(),
                expected_tools: vec![],
                status: PlanStepStatus::Pending,
                result: None,
            }],
            current_step: 0,
        };
        assert!(!plan.is_complete());
    }

    #[test]
    fn parse_plan_response_handles_markdown_fences() {
        let json = r#"```json
{"goal": "test", "steps": [{"description": "do X", "expected_tools": [], "status": "pending", "result": null}]}
```"#;
        let plan = parse_plan_response(json).unwrap();
        assert_eq!(plan.goal, "test");
        assert_eq!(plan.steps.len(), 1);
    }

    #[test]
    fn parse_plan_response_plain_json() {
        let json = r#"{"goal": "test", "steps": [{"description": "do X", "expected_tools": ["read_file"], "status": "pending", "result": null}]}"#;
        let plan = parse_plan_response(json).unwrap();
        assert_eq!(plan.steps[0].expected_tools, vec!["read_file"]);
    }

    #[test]
    fn parse_plan_rejects_empty_steps() {
        let json = r#"{"goal": "test", "steps": []}"#;
        let result = parse_plan_response(json);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn generate_plan_uses_mock_provider() {
        let mock = Arc::new(MockProvider::new());
        mock.enqueue_text(r#"{"goal":"read file","steps":[{"description":"read config","expected_tools":["read_file"],"status":"pending","result": null}]}"#).enqueue_done();

        let plan = generate_plan(&mock, "mock-model", "read config.toml", &["read_file", "write_file"])
            .await
            .unwrap();
        assert_eq!(plan.goal, "read file");
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].expected_tools, vec!["read_file"]);
    }

    #[test]
    fn format_plan_renders_status_markers() {
        let plan = AgentPlan {
            goal: "test goal".into(),
            steps: vec![
                PlanStep {
                    description: "step 1".into(),
                    expected_tools: vec!["read_file".into()],
                    status: PlanStepStatus::Done,
                    result: Some("ok".into()),
                },
                PlanStep {
                    description: "step 2".into(),
                    expected_tools: vec![],
                    status: PlanStepStatus::Pending,
                    result: None,
                },
            ],
            current_step: 1,
        };
        let formatted = format_plan(&plan);
        assert!(formatted.contains("✓"));
        assert!(formatted.contains("○"));
        assert!(formatted.contains("test goal"));
    }

    #[test]
    fn plan_advance_increments_step() {
        let mut plan = AgentPlan {
            goal: "test".into(),
            steps: vec![
                PlanStep {
                    description: "s1".into(),
                    expected_tools: vec![],
                    status: PlanStepStatus::Done,
                    result: None,
                },
                PlanStep {
                    description: "s2".into(),
                    expected_tools: vec![],
                    status: PlanStepStatus::Pending,
                    result: None,
                },
            ],
            current_step: 0,
        };
        plan.advance();
        assert_eq!(plan.current_step, 1);
    }
}
