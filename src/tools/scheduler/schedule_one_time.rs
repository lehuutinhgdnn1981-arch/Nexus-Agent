//! schedule_one_time tool.

use async_trait::async_trait;

use crate::error::{NexusError, Result};
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};

pub struct ScheduleOneTimeTool;

#[async_trait]
impl Tool for ScheduleOneTimeTool {
    fn name(&self) -> &'static str { "schedule_one_time" }

    fn description(&self) -> &'static str {
        "Schedule a one-time reminder using natural language (e.g. 'in 2 hours', 'tomorrow 9am')."
    }

    fn permission(&self) -> PermissionLevel { PermissionLevel::RequiresApproval }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "schedule": { "type": "string", "description": "Natural language time spec." },
                "message": { "type": "string", "description": "Message to inject into agent when fired." }
            },
            "required": ["schedule", "message"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> Result<ToolResult> {
        let schedule = input.get("schedule").and_then(|v| v.as_str())
            .ok_or_else(|| NexusError::InvalidArgument("missing `schedule`".into()))?;
        let message = input.get("message").and_then(|v| v.as_str())
            .ok_or_else(|| NexusError::InvalidArgument("missing `message`".into()))?;

        let id = ctx.scheduler.add_from_natural_language(schedule, message, ctx.session_id.as_deref()).await?;
        Ok(ToolResult::ok("", self.name(), format!("scheduled one-time job: {id}")))
    }
}
