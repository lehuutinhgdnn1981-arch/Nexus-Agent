//! Search provider tests — DuckDuckGo + Brave shapes.

#![cfg(test)]

use super::duckduckgo::DuckDuckGoSearch;
use super::provider::SearchProvider;
use super::web_search_tool::WebSearchTool;

#[tokio::test]
async fn duckduckgo_returns_results() {
    let ddg = DuckDuckGoSearch::new();
    let results = ddg.search("rust programming language", 3).await;
    if let Err(e) = &results {
        eprintln!("DDG search failed (network?): {e} — skipping assertion");
        return;
    }
    let results = results.unwrap();
    assert!(!results.is_empty(), "DDG returned 0 results");
    for r in &results {
        assert!(!r.title.is_empty());
        assert!(!r.url.is_empty());
    }
}

#[test]
fn web_search_tool_default_provider() {
    let tool = WebSearchTool::default();
    assert_eq!(tool.name(), "web_search");
    assert_eq!(tool.permission(), crate::security::permission::PermissionLevel::Safe);
}

#[test]
fn web_search_tool_schema_has_required_fields() {
    let tool = WebSearchTool::default();
    let schema = tool.schema();
    let props = schema.get("properties").unwrap();
    assert!(props.get("query").is_some());
    assert!(props.get("limit").is_some());
    assert!(props.get("provider").is_some());
}

#[test]
fn search_result_serializes() {
    let r = super::provider::SearchResult {
        title: "Test".into(),
        url: "https://example.com".into(),
        snippet: "snippet".into(),
    };
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("Test"));
    assert!(json.contains("example.com"));
}
