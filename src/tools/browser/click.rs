//! browser_click tool.

use async_trait::async_trait;

use crate::browser::page::{execute, PageAction};
use crate::error::{NexusError, Result};
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};

pub struct BrowserClickTool;

#[async_trait]
impl Tool for BrowserClickTool {
    fn name(&self) -> &'static str { "browser_click" }

    fn description(&self) -> &'static str { "Click an element matching a CSS selector." }

    fn permission(&self) -> PermissionLevel { PermissionLevel::RequiresApproval }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "selector": { "type": "string", "description": "CSS selector." },
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
        let result = execute(&page, &PageAction::Click { selector: selector.to_string(), timeout_ms }).await?;
        Ok(ToolResult::ok("", self.name(), result.to_string()).with_data(result))
    }
}
