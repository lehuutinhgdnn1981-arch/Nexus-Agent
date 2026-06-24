//! browser_wait tool.

use async_trait::async_trait;

use crate::browser::page::{execute, PageAction};
use crate::error::{NexusError, Result};
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};

pub struct BrowserWaitTool;

#[async_trait]
impl Tool for BrowserWaitTool {
    fn name(&self) -> &'static str { "browser_wait" }

    fn description(&self) -> &'static str { "Wait until an element matching a CSS selector appears." }

    fn permission(&self) -> PermissionLevel { PermissionLevel::Safe }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "selector": { "type": "string" },
                "timeout_ms": { "type": "integer", "default": 30000 }
            },
            "required": ["selector"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> Result<ToolResult> {
        let selector = input.get("selector").and_then(|v| v.as_str())
            .ok_or_else(|| NexusError::InvalidArgument("missing `selector`".into()))?;
        let timeout_ms = input.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(30_000);

        let page = ctx.browser.page().await?;
        let result = execute(&page, &PageAction::Wait { selector: selector.to_string(), timeout_ms }).await?;
        Ok(ToolResult::ok("", self.name(), result.to_string()).with_data(result))
    }
}
