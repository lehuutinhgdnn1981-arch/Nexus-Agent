//! Tool trait + result types.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{NexusError, Result};
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;

/// Tool call (LLM-issued).
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ToolCall {
    pub id: String,
    #[serde(default = "default_tool_type")]
    pub r#type: String,
    pub function: ToolCallFunction,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}

fn default_tool_type() -> String {
    "function".into()
}

/// Tool execution result.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ToolResult {
    pub call_id: String,
    pub tool: String,
    pub ok: bool,
    pub output: String,
    /// Optional structured data (JSON string).
    #[serde(default)]
    #[ts(type = "any")]

    pub data: Option<serde_json::Value>,
    /// Execution duration in millis.
    #[serde(default)]
    pub duration_ms: u64,
}

impl ToolResult {
    #[must_use]
    pub fn ok(call_id: &str, tool: &str, output: impl Into<String>) -> Self {
        Self {
            call_id: call_id.to_string(),
            tool: tool.to_string(),
            ok: true,
            output: output.into(),
            data: None,
            duration_ms: 0,
        }
    }

    #[must_use]
    pub fn error(call_id: &str, tool: &str, output: impl Into<String>) -> Self {
        Self {
            call_id: call_id.to_string(),
            tool: tool.to_string(),
            ok: false,
            output: output.into(),
            data: None,
            duration_ms: 0,
        }
    }

    #[must_use]
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    #[must_use]
    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }
}

/// Tool error — wrapped trong NexusError::Tool.
pub type ToolError = NexusError;

/// The Tool trait — mọi tool implement interface này.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tên tool (unique trong registry).
    fn name(&self) -> &'static str;

    /// Mô tả ngắn cho LLM.
    fn description(&self) -> &'static str;

    /// Permission level — quyết định có cần approval không.
    fn permission(&self) -> PermissionLevel;

    /// JSON Schema cho parameters.
    fn schema(&self) -> serde_json::Value;

    /// Execute tool với input JSON.
    async fn execute(
        &self,
        ctx: &ToolContext,
        input: serde_json::Value,
    ) -> Result<ToolResult>;
}
