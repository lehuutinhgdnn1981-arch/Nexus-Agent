//! write_file tool.

use async_trait::async_trait;

use crate::error::{NexusError, Result};
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};

pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &'static str {
        "write_file"
    }

    fn description(&self) -> &'static str {
        "Write text content to a file inside the workspace sandbox. Creates parent directories if needed. Overwrites existing content."
    }

    fn permission(&self) -> PermissionLevel {
        PermissionLevel::RequiresApproval
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path relative to workspace root." },
                "content": { "type": "string", "description": "Text content to write." }
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
        std::fs::write(&resolved, content)?;
        Ok(ToolResult::ok("", self.name(), format!("wrote {} bytes to {}", content.len(), resolved.display())))
    }
}
