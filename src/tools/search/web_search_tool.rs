//! web_search tool — adapter chọn SearchProvider runtime.

use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

use crate::error::{NexusError, Result};
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::search::brave::BraveSearch;
use crate::tools::search::duckduckgo::DuckDuckGoSearch;
use crate::tools::search::provider::{SearchProvider, SearchResult};
use crate::tools::tool::{Tool, ToolResult};

pub struct WebSearchTool {
    /// Provider mặc định. Nếu None, sẽ tự động chọn theo config.
    default: Option<String>,
}

impl WebSearchTool {
    #[must_use]
    pub fn new() -> Self {
        Self { default: None }
    }

    #[must_use]
    pub fn with_default(provider: impl Into<String>) -> Self {
        Self {
            default: Some(provider.into()),
        }
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

fn resolve_provider(ctx: &ToolContext, preferred: Option<&str>) -> Arc<dyn SearchProvider> {
    let chosen = preferred
        .or(ctx.config.search.default.as_str().into())
        .unwrap_or("duckduckgo");

    match chosen {
        "brave" => {
            if let Some(p) = BraveSearch::from_config(ctx.config.search.brave_api_key.as_deref()) {
                Arc::new(p)
            } else {
                Arc::new(DuckDuckGoSearch::new())
            }
        }
        _ => Arc::new(DuckDuckGoSearch::new()),
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &'static str { "web_search" }

    fn description(&self) -> &'static str {
        "Search the web via DuckDuckGo (default, no API key) or Brave (requires API key)."
    }

    fn permission(&self) -> PermissionLevel { PermissionLevel::Safe }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" },
                "limit": { "type": "integer", "default": 5, "maximum": 20 },
                "provider": { "type": "string", "enum": ["duckduckgo", "brave"], "description": "Override default provider." }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> Result<ToolResult> {
        let query = input.get("query").and_then(|v| v.as_str())
            .ok_or_else(|| NexusError::InvalidArgument("missing `query`".into()))?;
        let limit = input.get("limit").and_then(|v| v.as_u64()).unwrap_or(5).min(20) as u32;
        let provider_str = input.get("provider").and_then(|v| v.as_str())
            .or(self.default.as_deref());

        let provider = resolve_provider(ctx, provider_str);
        let results: Vec<SearchResult> = provider.search(query, limit).await?;

        let text = results
            .iter()
            .enumerate()
            .map(|(i, r)| format!("{}. {} — {}\n   {}", i + 1, r.title, r.url, r.snippet))
            .collect::<Vec<_>>()
            .join("\n\n");

        Ok(ToolResult::ok("", self.name(), if text.is_empty() {
            "no results".to_string()
        } else {
            text
        })
        .with_data(json!({ "provider": provider.id(), "results": results })))
    }
}
