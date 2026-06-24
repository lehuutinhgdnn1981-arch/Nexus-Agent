//! cancel_scheduled tool.

use async_trait::async_trait;

use crate::error::{NexusError, Result};
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};

pub struct CancelScheduledTool;

#[async_trait]
impl Tool for CancelScheduledTool {
    fn name(&self) -> &'static str { "cancel_scheduled" }

    fn description(&self) -> &'static str { "Cancel a scheduled job by ID. DANGEROUS — affects user's reminders." }

    fn permission(&self) -> PermissionLevel { PermissionLevel::Dangerous }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": { "id": { "type": "string" } },
            "required": ["id"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> Result<ToolResult> {
        let id = input.get("id").and_then(|v| v.as_str())
            .ok_or_else(|| NexusError::InvalidArgument("missing `id`".into()))?;
        ctx.scheduler.cancel(id).await?;
        Ok(ToolResult::ok("", self.name(), format!("cancelled job {id}")))
    }
}
