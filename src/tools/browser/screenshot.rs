//! browser_screenshot tool.

use async_trait::async_trait;

use crate::browser::page::{execute, PageAction};
use crate::error::Result;
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};

pub struct BrowserScreenshotTool;

#[async_trait]
impl Tool for BrowserScreenshotTool {
    fn name(&self) -> &'static str { "browser_screenshot" }

    fn description(&self) -> &'static str { "Capture a PNG screenshot of the current page (full or viewport)." }

    fn permission(&self) -> PermissionLevel { PermissionLevel::Safe }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "full_page": { "type": "boolean", "default": false }
            }
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> Result<ToolResult> {
        let full_page = input.get("full_page").and_then(|v| v.as_bool()).unwrap_or(false);
        let page = ctx.browser.page().await?;
        let result = execute(&page, &PageAction::Screenshot { full_page }).await?;
        Ok(ToolResult::ok("", self.name(), "screenshot captured").with_data(result))
    }
}
