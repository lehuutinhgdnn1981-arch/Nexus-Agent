//! schedule_recurring tool.

use async_trait::async_trait;

use crate::error::{NexusError, Result};
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};

pub struct ScheduleRecurringTool;

#[async_trait]
impl Tool for ScheduleRecurringTool {
    fn name(&self) -> &'static str { "schedule_recurring" }

    fn description(&self) -> &'static str {
        "Schedule a recurring reminder using natural language (e.g. 'every day 9am', 'every weekday 8:30')."
    }

    fn permission(&self) -> PermissionLevel { PermissionLevel::RequiresApproval }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "schedule": { "type": "string" },
                "message": { "type": "string" }
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
        Ok(ToolResult::ok("", self.name(), format!("scheduled recurring job: {id}")))
    }
}
