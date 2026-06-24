//! delete_file tool.

use async_trait::async_trait;
use std::fs;

use crate::error::{NexusError, Result};
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};

pub struct DeleteFileTool;

#[async_trait]
impl Tool for DeleteFileTool {
    fn name(&self) -> &'static str {
        "delete_file"
    }

    fn description(&self) -> &'static str {
        "Delete a file or empty directory inside the workspace sandbox. DANGEROUS — irreversible."
    }

    fn permission(&self) -> PermissionLevel {
        PermissionLevel::Dangerous
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to delete." }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> Result<ToolResult> {
        let path_str = input.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| NexusError::InvalidArgument("missing `path`".into()))?;
        let resolved = ctx.workspace.resolve(path_str)?;

        let metadata = fs::metadata(&resolved)?;
        if metadata.is_dir() {
            // Chỉ cho phép xóa empty directory
            let mut entries = fs::read_dir(&resolved)?;
            if entries.next().is_some() {
                return Ok(ToolResult::error("", self.name(), "directory is not empty — refusing to delete non-empty dir"));
            }
            fs::remove_dir(&resolved)?;
        } else {
            fs::remove_file(&resolved)?;
        }
        Ok(ToolResult::ok("", self.name(), format!("deleted {}", resolved.display())))
    }
}
