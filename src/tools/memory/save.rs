//! memory_save tool.

use async_trait::async_trait;

use crate::error::{NexusError, Result};
use crate::memory::model::MemoryCategory;
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};

pub struct MemorySaveTool;

#[async_trait]
impl Tool for MemorySaveTool {
    fn name(&self) -> &'static str { "memory_save" }

    fn description(&self) -> &'static str {
        "Save a long-term memory entry (fact/preference/task/note). Deduplicated by embedding similarity."
    }

    fn permission(&self) -> PermissionLevel { PermissionLevel::RequiresApproval }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "content": { "type": "string" },
                "category": { "type": "string", "enum": ["fact", "preference", "task", "note"] },
                "tags": { "type": "array", "items": { "type": "string" }, "default": [] }
            },
            "required": ["content", "category"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> Result<ToolResult> {
        let content = input.get("content").and_then(|v| v.as_str())
            .ok_or_else(|| NexusError::InvalidArgument("missing `content`".into()))?;
        let category_str = input.get("category").and_then(|v| v.as_str())
            .ok_or_else(|| NexusError::InvalidArgument("missing `category`".into()))?;
        let tags: Vec<String> = input.get("tags").and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let category = MemoryCategory::from_str(category_str);
        let id = ctx.memory.save_long_term(content, category, tags, ctx.session_id.as_deref()).await?;
        Ok(ToolResult::ok("", self.name(), format!("memory saved: {id}")))
    }
}
