//! read_file tool.

use async_trait::async_trait;
use std::path::PathBuf;

use crate::error::{NexusError, Result};
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};

pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &'static str {
        "read_file"
    }

    fn description(&self) -> &'static str {
        "Read text content of a file inside the workspace sandbox."
    }

    fn permission(&self) -> PermissionLevel {
        PermissionLevel::Safe
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path relative to workspace root, or absolute path inside sandbox."
                },
                "max_bytes": {
                    "type": "integer",
                    "description": "Maximum bytes to read (default 65536).",
                    "default": 65536
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> Result<ToolResult> {
        let path_str = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NexusError::InvalidArgument("missing `path`".into()))?;
        let max_bytes = input
            .get("max_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(65_536) as usize;

        let resolved: PathBuf = ctx.workspace.resolve(path_str)?;
        let metadata = std::fs::metadata(&resolved)?;
        if metadata.len() > (max_bytes as u64) * 2 {
            // Hard limit: 2× max_bytes — refuse oversized files
            return Ok(ToolResult::error(
                "",
                self.name(),
                format!(
                    "file is {} bytes, exceeds 2× max_bytes ({})",
                    metadata.len(),
                    max_bytes * 2
                ),
            ));
        }
        let content = std::fs::read_to_string(&resolved)?;
        let truncated = if content.len() > max_bytes {
            content[..max_bytes].to_string() + "\n...[truncated]"
        } else {
            content
        };
        Ok(ToolResult::ok("", self.name(), truncated))
    }
}
