//! Agent event types — emitted qua mpsc channel + IPC events.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::llm::types::Usage;
use crate::tools::tool::ToolResult;

/// Event từ agent loop, gửi tới frontend qua IPC.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentEvent {
    /// Bắt đầu 1 turn mới.
    TurnStart {
        run_id: String,
        session_id: String,
        user_message: String,
    },
    /// Stream token từ LLM.
    Delta {
        run_id: String,
        session_id: String,
        text: String,
    },
    /// Tool call bắt đầu (sau khi approved nếu cần).
    ToolCallStart {
        run_id: String,
        call_id: String,
        tool: String,
        input: serde_json::Value,
    },
    /// Tool call hoàn thành.
    ToolCallEnd {
        run_id: String,
        call_id: String,
        result: ToolResult,
    },
    /// 1 iteration hoàn thành (reasoning → tool exec → observe).
    IterationEnd {
        run_id: String,
        iteration: u32,
        tool_calls_made: u32,
    },
    /// Turn kết thúc thành công.
    Done {
        run_id: String,
        session_id: String,
        final_message: String,
        usage: Usage,
    },
    /// Turn lỗi.
    Error {
        run_id: String,
        message: String,
    },
    /// Turn bị user cancel.
    Cancelled {
        run_id: String,
    },
    /// Approval request — frontend phải show dialog.
    ApprovalRequest {
        run_id: String,
        request_id: String,
        tool: String,
        input: serde_json::Value,
        permission: String,
    },
}

impl AgentEvent {
    #[must_use]
    pub fn run_id(&self) -> &str {
        match self {
            Self::TurnStart { run_id, .. }
            | Self::Delta { run_id, .. }
            | Self::ToolCallStart { run_id, .. }
            | Self::ToolCallEnd { run_id, .. }
            | Self::IterationEnd { run_id, .. }
            | Self::Done { run_id, .. }
            | Self::Error { run_id, .. }
            | Self::Cancelled { run_id, .. }
            | Self::ApprovalRequest { run_id, .. } => run_id,
        }
    }
}
