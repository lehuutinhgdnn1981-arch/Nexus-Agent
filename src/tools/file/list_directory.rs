//! list_directory tool.

use async_trait::async_trait;
use serde_json::json;
use std::fs;

use crate::error::{NexusError, Result};
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};

pub struct ListDirectoryTool;

#[async_trait]
impl Tool for ListDirectoryTool {
    fn name(&self) -> &'static str { "list_directory" }

    fn description(&self) -> &'static str {
        "List entries in a directory inside the workspace sandbox. Returns name + type + size for each."
    }

    fn permission(&self) -> PermissionLevel { PermissionLevel::Safe }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Directory path. Defaults to workspace root." },
                "include_hidden": { "type": "boolean", "default": false }
            },
            "required": []
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> Result<ToolResult> {
        let path_str = input.get("path").and_then(|v| v.as_str()).unwrap_or("");
        let include_hidden = input.get("include_hidden").and_then(|v| v.as_bool()).unwrap_or(false);

        let resolved = if path_str.is_empty() {
            ctx.workspace.root().to_path_buf()
        } else {
            ctx.workspace.resolve(path_str)?
        };

        let mut entries_out = Vec::new();
        for entry in fs::read_dir(&resolved)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if !include_hidden && name.starts_with('.') {
                continue;
            }
            let metadata = entry.metadata()?;
            entries_out.push(json!({
                "name": name,
                "type": if metadata.is_dir() { "dir" } else { "file" },
                "size": metadata.len(),
            }));
        }
        entries_out.sort_by(|a, b| {
            let a_type = a.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let b_type = b.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let a_name = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let b_name = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
            (a_type, a_name).cmp(&(b_type, b_name))
        });
        let count = entries_out.len();
        Ok(ToolResult::ok("", self.name(), format!("listed {count} entries"))
            .with_data(json!({ "entries": entries_out, "path": resolved.display().to_string() })))
    }
}
