//! browser_navigate tool.

use async_trait::async_trait;

use crate::browser::page::{execute, PageAction};
use crate::error::{NexusError, Result};
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};

pub struct BrowserNavigateTool;

#[async_trait]
impl Tool for BrowserNavigateTool {
    fn name(&self) -> &'static str { "browser_navigate" }

    fn description(&self) -> &'static str { "Navigate the browser tab to a URL." }

    fn permission(&self) -> PermissionLevel { PermissionLevel::Safe }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "URL to navigate to." },
                "timeout_ms": { "type": "integer", "default": 30000 }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> Result<ToolResult> {
        let url = input.get("url").and_then(|v| v.as_str())
            .ok_or_else(|| NexusError::InvalidArgument("missing `url`".into()))?;
        let timeout_ms = input.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(30_000);

        let page = ctx.browser.page().await?;
        let result = execute(&page, &PageAction::Navigate { url: url.to_string(), timeout_ms }).await?;
        Ok(ToolResult::ok("", self.name(), result.to_string()).with_data(result))
    }
}
