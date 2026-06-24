//! create_directory tool.

use async_trait::async_trait;
use std::fs;

use crate::error::{NexusError, Result};
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};

pub struct CreateDirectoryTool;

#[async_trait]
impl Tool for CreateDirectoryTool {
    fn name(&self) -> &'static str { "create_directory" }

    fn description(&self) -> &'static str {
        "Create a directory (and parents) inside the workspace sandbox."
    }

    fn permission(&self) -> PermissionLevel { PermissionLevel::RequiresApproval }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> Result<ToolResult> {
        let path_str = input.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| NexusError::InvalidArgument("missing `path`".into()))?;
        let resolved = ctx.workspace.resolve(path_str)?;
        fs::create_dir_all(&resolved)?;
        Ok(ToolResult::ok("", self.name(), format!("created dir {}", resolved.display())))
    }
}
