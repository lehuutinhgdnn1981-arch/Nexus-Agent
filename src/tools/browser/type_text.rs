//! browser_type tool.

use async_trait::async_trait;

use crate::browser::page::{execute, PageAction};
use crate::error::{NexusError, Result};
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};

pub struct BrowserTypeTool;

#[async_trait]
impl Tool for BrowserTypeTool {
    fn name(&self) -> &'static str { "browser_type" }

    fn description(&self) -> &'static str { "Type text into an element matching a CSS selector." }

    fn permission(&self) -> PermissionLevel { PermissionLevel::RequiresApproval }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "selector": { "type": "string" },
                "text": { "type": "string" },
                "delay_ms": { "type": "integer", "default": 0 }
            },
            "required": ["selector", "text"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> Result<ToolResult> {
        let selector = input.get("selector").and_then(|v| v.as_str())
            .ok_or_else(|| NexusError::InvalidArgument("missing `selector`".into()))?;
        let text = input.get("text").and_then(|v| v.as_str())
            .ok_or_else(|| NexusError::InvalidArgument("missing `text`".into()))?;
        let delay_ms = input.get("delay_ms").and_then(|v| v.as_u64()).unwrap_or(0);

        let page = ctx.browser.page().await?;
        let result = execute(&page, &PageAction::Type {
            selector: selector.to_string(),
            text: text.to_string(),
            delay_ms,
        }).await?;
        Ok(ToolResult::ok("", self.name(), result.to_string()).with_data(result))
    }
}
