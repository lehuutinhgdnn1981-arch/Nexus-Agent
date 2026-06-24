//! Web search tools.

pub mod brave;
pub mod duckduckgo;
pub mod provider;
pub mod web_search_tool;

#[cfg(test)]
mod tests;

pub use brave::BraveSearch;
pub use duckduckgo::DuckDuckGoSearch;
pub use provider::{SearchProvider, SearchResult};

pub fn register_all(registry: &crate::tools::registry::ToolRegistry) {
    registry.register(web_search_tool::WebSearchTool::default());
}
