//! memory_recall tool.

use async_trait::async_trait;
use serde_json::json;

use crate::error::{NexusError, Result};
use crate::memory::model::{MemoryCategory, MemoryQuery};
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};

pub struct MemoryRecallTool;

#[async_trait]
impl Tool for MemoryRecallTool {
    fn name(&self) -> &'static str { "memory_recall" }

    fn description(&self) -> &'static str { "Recall top-K long-term memories relevant to a query (cosine similarity)." }

    fn permission(&self) -> PermissionLevel { PermissionLevel::Safe }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" },
                "top_k": { "type": "integer", "default": 5 },
                "category": { "type": "string", "enum": ["fact", "preference", "task", "note"] }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> Result<ToolResult> {
        let query_str = input.get("query").and_then(|v| v.as_str())
            .ok_or_else(|| NexusError::InvalidArgument("missing `query`".into()))?;
        let top_k = input.get("top_k").and_then(|v| v.as_u64()).unwrap_or(5) as u32;
        let category = input.get("category").and_then(|v| v.as_str()).map(MemoryCategory::from_str);

        let mut q = MemoryQuery::new(query_str);
        q.top_k = top_k;
        q.category = category;

        let results = ctx.memory.recall(&q).await?;
        let text = results.iter()
            .enumerate()
            .map(|(i, m)| format!("{}. [{}] {}", i + 1, m.category.as_str(), m.content))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(ToolResult::ok("", self.name(), if text.is_empty() { "no memories found".into() } else { text })
            .with_data(json!({ "count": results.len(), "memories": results })))
    }
}
