//! list_scheduled tool.

use async_trait::async_trait;
use serde_json::json;

use crate::error::Result;
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};

pub struct ListScheduledTool;

#[async_trait]
impl Tool for ListScheduledTool {
    fn name(&self) -> &'static str { "list_scheduled" }

    fn description(&self) -> &'static str { "List all scheduled jobs." }

    fn permission(&self) -> PermissionLevel { PermissionLevel::Safe }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({ "type": "object", "properties": {} })
    }

    async fn execute(&self, ctx: &ToolContext, _input: serde_json::Value) -> Result<ToolResult> {
        let jobs = ctx.scheduler.list().await?;
        let text = jobs.iter()
            .map(|j| format!("- {} [enabled={}] {:?}: {}", j.id, j.enabled, j.kind, j.message))
            .collect::<Vec<_>>()
            .join("\n");
        Ok(ToolResult::ok("", self.name(), if text.is_empty() { "no jobs".into() } else { text })
            .with_data(json!({ "count": jobs.len(), "jobs": jobs })))
    }
}
