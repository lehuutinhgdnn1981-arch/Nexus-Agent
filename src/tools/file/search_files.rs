//! search_files tool — recursive glob search.

use async_trait::async_trait;
use serde_json::json;

use crate::error::{NexusError, Result};
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};

pub struct SearchFilesTool;

#[async_trait]
impl Tool for SearchFilesTool {
    fn name(&self) -> &'static str { "search_files" }

    fn description(&self) -> &'static str {
        "Search for files matching a glob pattern inside the workspace sandbox. Returns list of matching paths."
    }

    fn permission(&self) -> PermissionLevel { PermissionLevel::Safe }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Glob pattern, e.g. `**/*.rs` or `**/notes.txt`." },
                "max_results": { "type": "integer", "default": 100 }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> Result<ToolResult> {
        let pattern = input.get("pattern").and_then(|v| v.as_str())
            .ok_or_else(|| NexusError::InvalidArgument("missing `pattern`".into()))?;
        let max_results = input.get("max_results").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

        let root = ctx.workspace.root().to_path_buf();
        let full_pattern = if pattern.starts_with('/') || pattern.starts_with('\\') {
            pattern.to_string()
        } else {
            format!("{}{}", root.display(), std::path::MAIN_SEPARATOR.to_string() + pattern)
        };

        let mut matches = Vec::new();
        for entry in glob::glob(&full_pattern)
            .map_err(|e| NexusError::InvalidArgument(format!("invalid glob: {e}")))?
        {
            if let Ok(path) = entry {
                let rel = path.strip_prefix(&root).unwrap_or(&path).to_string_lossy().to_string();
                matches.push(rel);
                if matches.len() >= max_results {
                    break;
                }
            }
        }

        let count = matches.len();
        Ok(ToolResult::ok("", self.name(), format!("found {count} matches"))
            .with_data(json!({ "matches": matches })))
    }
}
