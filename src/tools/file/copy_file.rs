//! copy_file tool.

use async_trait::async_trait;
use std::fs;

use crate::error::{NexusError, Result};
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};

pub struct CopyFileTool;

#[async_trait]
impl Tool for CopyFileTool {
    fn name(&self) -> &'static str { "copy_file" }

    fn description(&self) -> &'static str {
        "Copy a file inside the workspace sandbox. Creates parent directories of destination if needed."
    }

    fn permission(&self) -> PermissionLevel { PermissionLevel::RequiresApproval }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "src": { "type": "string" },
                "dst": { "type": "string" }
            },
            "required": ["src", "dst"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> Result<ToolResult> {
        let src = input.get("src").and_then(|v| v.as_str())
            .ok_or_else(|| NexusError::InvalidArgument("missing `src`".into()))?;
        let dst = input.get("dst").and_then(|v| v.as_str())
            .ok_or_else(|| NexusError::InvalidArgument("missing `dst`".into()))?;

        let src_path = ctx.workspace.resolve(src)?;
        let dst_path = ctx.workspace.resolve_with_parents(dst)?;
        fs::copy(&src_path, &dst_path)?;
        Ok(ToolResult::ok("", self.name(), format!("copied {} → {}", src_path.display(), dst_path.display())))
    }
}
