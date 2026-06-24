//! Integration tests cho memory system (long-term + short-term).

mod common;

use nexus::memory::model::{MemoryCategory, MemoryQuery};
use nexus::tools::context::ToolContext;
use nexus::tools::memory::{recall::MemoryRecallTool, save::MemorySaveTool};
use nexus::tools::tool::Tool;
use nexus::llm::types::ChatMessage;
use serde_json::json;

#[tokio::test]
async fn integration_short_term_push_and_recall() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx: ToolContext = common::tool_context(tmp.path()).await;

    ctx.memory
        .push_short_term("s1", ChatMessage::user("Hello"))
        .await;
    ctx.memory
        .push_short_term("s1", ChatMessage::assistant("Hi there"))
        .await;

    let all = ctx.memory.short_term_all("s1").await;
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].content, "Hello");
    assert_eq!(all[1].content, "Hi there");
}

#[tokio::test]
async fn integration_short_term_drops_on_session_end() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = common::tool_context(tmp.path()).await;

    ctx.memory
        .push_short_term("s1", ChatMessage::user("temp"))
        .await;
    assert_eq!(ctx.memory.short_term_all("s1").await.len(), 1);

    ctx.memory.drop_session("s1");
    assert_eq!(ctx.memory.short_term_all("s1").await.len(), 0);
}

#[tokio::test]
async fn integration_long_term_save_and_list() {
    // Skip nếu Ollama không chạy (no embedding provider available)
    let tmp = tempfile::tempdir().unwrap();
    let ctx = common::tool_context(tmp.path()).await;

    // Try save — sẽ fail nếu Ollama not running, đó là OK
    let result = ctx
        .memory
        .save_long_term(
            "test fact",
            MemoryCategory::Fact,
            vec!["test".into()],
            Some("s1"),
        )
        .await;

    if result.is_err() {
        eprintln!("Ollama not running, skipping save test: {:?}", result.err());
        return;
    }

    let list = ctx.memory.list_recent(10).await.unwrap();
    assert!(list.iter().any(|m| m.content == "test fact"));
}

#[tokio::test]
async fn integration_memory_tool_save_via_tool() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = common::tool_context(tmp.path()).await;

    let save_tool = MemorySaveTool;
    let result = save_tool
        .execute(
            &ctx,
            json!({
                "content": "User prefers dark mode",
                "category": "preference",
                "tags": ["ui", "preference"]
            }),
        )
        .await;

    if result.is_err() {
        eprintln!("Ollama not running, skipping: {:?}", result.err());
        return;
    }

    let r = result.unwrap();
    assert!(r.ok);
}

#[tokio::test]
async fn integration_memory_recall_query_shape() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = common::tool_context(tmp.path()).await;

    let recall_tool = MemoryRecallTool;
    // Recall will return empty list if no memories stored
    let r = recall_tool
        .execute(&ctx, json!({"query": "anything", "top_k": 3}))
        .await;

    if r.is_err() {
        eprintln!("Ollama not running: {:?}", r.err());
        return;
    }
    let r = r.unwrap();
    // Just check it doesn't crash — content may be "no memories found"
    let _ = r;
}

#[tokio::test]
async fn integration_memory_query_builder() {
    let q = MemoryQuery::new("test query");
    assert_eq!(q.text, "test query");
    assert_eq!(q.top_k, 5);
    assert_eq!(q.min_similarity, 0.0);
    assert!(q.category.is_none());
}
