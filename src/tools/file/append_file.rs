//! append_file tool.

use async_trait::async_trait;
use std::io::Write;

use crate::error::{NexusError, Result};
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};

pub struct AppendFileTool;

#[async_trait]
impl Tool for AppendFileTool {
    fn name(&self) -> &'static str {
        "append_file"
    }

    fn description(&self) -> &'static str {
        "Append text to a file inside the workspace sandbox. Creates the file if it does not exist."
    }

    fn permission(&self) -> PermissionLevel {
        PermissionLevel::RequiresApproval
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "content": { "type": "string" }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> Result<ToolResult> {
        let path_str = input.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| NexusError::InvalidArgument("missing `path`".into()))?;
        let content = input.get("content").and_then(|v| v.as_str())
            .ok_or_else(|| NexusError::InvalidArgument("missing `content`".into()))?;

        let resolved = ctx.workspace.resolve_with_parents(path_str)?;
        let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&resolved)?;
        file.write_all(content.as_bytes())?;
        Ok(ToolResult::ok("", self.name(), format!("appended {} bytes to {}", content.len(), resolved.display())))
    }
}
