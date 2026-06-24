//! browser_extract_text tool.

use async_trait::async_trait;

use crate::browser::page::{execute, PageAction};
use crate::error::Result;
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};

pub struct BrowserExtractTextTool;

#[async_trait]
impl Tool for BrowserExtractTextTool {
    fn name(&self) -> &'static str { "browser_extract_text" }

    fn description(&self) -> &'static str { "Extract visible text content from the current page." }

    fn permission(&self) -> PermissionLevel { PermissionLevel::Safe }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({ "type": "object", "properties": {} })
    }

    async fn execute(&self, ctx: &ToolContext, _input: serde_json::Value) -> Result<ToolResult> {
        let page = ctx.browser.page().await?;
        let result = execute(&page, &PageAction::ExtractText).await?;
        Ok(ToolResult::ok("", self.name(), result.to_string()).with_data(result))
    }
}
