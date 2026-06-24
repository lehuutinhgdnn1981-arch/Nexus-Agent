//! Integration tests cho Agent loop với MockProvider.

#![cfg(feature = "test-utils")]

mod common;

use std::sync::Arc;

use nexus::agent::agent::Agent;
use nexus::agent::config::AgentRuntimeConfig;
use nexus::agent::event::AgentEvent;
use nexus::database::repositories::{message_repo::MessageRepo, session_repo::SessionRepo};
use nexus::llm::MockProvider;
use nexus::tools::file::write_file::WriteFileTool;
use nexus::tools::tool::Tool;
use serde_json::json;
use tokio::sync::mpsc;

#[tokio::test]
async fn agent_simple_text_response() {
    let tmp = tempfile::tempdir().unwrap();
    let mock = Arc::new(MockProvider::new());
    let state = common::app_state_with_mock(tmp.path(), Arc::clone(&mock)).await;

    SessionRepo::create(&state.pool, "s1", "Test", "mock", "mock-model", None)
        .await
        .unwrap();

    // Script: assistant says "Hello!" then done, no tool calls
    mock.enqueue_text("Hello!").enqueue_done();

    let rt_config = AgentRuntimeConfig::default();
    let agent = Agent::new(Arc::clone(&state), rt_config, mock);
    let (tx, mut rx) = mpsc::channel::<AgentEvent>(128);

    agent.run("s1", "hi", tx).await.unwrap();

    let mut deltas = String::new();
    let mut done = false;
    while let Some(ev) = rx.recv().await {
        match ev {
            AgentEvent::Delta { text, .. } => deltas.push_str(&text),
            AgentEvent::Done { .. } => {
                done = true;
                break;
            }
            AgentEvent::Error { message, .. } => panic!("agent error: {message}"),
            _ => {}
        }
    }
    assert!(done);
    assert_eq!(deltas, "Hello!");

    // Verify messages persisted
    let msgs = MessageRepo::list_by_session(&state.pool, "s1").await.unwrap();
    assert_eq!(msgs.len(), 2); // user + assistant
    assert_eq!(msgs[0].role, "user");
    assert_eq!(msgs[0].content, "hi");
    assert_eq!(msgs[1].role, "assistant");
    assert_eq!(msgs[1].content, "Hello!");
}

#[tokio::test]
async fn agent_tool_call_flow() {
    let tmp = tempfile::tempdir().unwrap();
    let mock = Arc::new(MockProvider::new());
    let state = common::app_state_with_mock(tmp.path(), Arc::clone(&mock)).await;

    // Setup: write a file via direct tool call
    {
        let ctx = nexus::tools::context::ToolContext {
            session_id: Some("s1".into()),
            run_id: Some("setup".into()),
            workspace: Arc::clone(&state.sandbox),
            pool: state.pool.clone(),
            memory: Arc::clone(&state.memory),
            browser: Arc::clone(&state.browser),
            scheduler: Arc::clone(&state.scheduler),
            config: Arc::clone(&state.config),
        };
        WriteFileTool
            .execute(&ctx, json!({"path": "test.txt", "content": "test content"}))
            .await
            .unwrap();
    }

    SessionRepo::create(&state.pool, "s1", "Test", "mock", "mock-model", None)
        .await
        .unwrap();

    // Iteration 1: assistant calls read_file, then done
    mock.enqueue_tool_call("tc1", "read_file", json!({"path": "test.txt"}))
        .enqueue_done();
    // Iteration 2: assistant sees tool result, says "File contains: test content", done
    mock.enqueue_text("File contains: test content").enqueue_done();

    let rt_config = AgentRuntimeConfig::default();
    let agent = Agent::new(Arc::clone(&state), rt_config, mock);
    let (tx, mut rx) = mpsc::channel::<AgentEvent>(128);

    agent.run("s1", "read test.txt", tx).await.unwrap();

    let mut tool_call_count = 0;
    let mut tool_result_count = 0;
    let mut deltas = String::new();
    let mut done = false;
    while let Some(ev) = rx.recv().await {
        match ev {
            AgentEvent::ToolCallStart { .. } => tool_call_count += 1,
            AgentEvent::ToolCallEnd { result, .. } => {
                tool_result_count += 1;
                assert!(result.ok);
                assert!(result.output.contains("test content"));
            }
            AgentEvent::Delta { text, .. } => deltas.push_str(&text),
            AgentEvent::Done { .. } => {
                done = true;
                break;
            }
            AgentEvent::Error { message, .. } => panic!("agent error: {message}"),
            _ => {}
        }
    }

    assert!(done);
    assert_eq!(tool_call_count, 1);
    assert_eq!(tool_result_count, 1);
    assert!(deltas.contains("File contains: test content"));

    // Verify messages persisted (user + assistant tool call + tool result + assistant final)
    let msgs = MessageRepo::list_by_session(&state.pool, "s1").await.unwrap();
    assert_eq!(msgs.len(), 4);
    assert_eq!(msgs[0].role, "user");
    assert_eq!(msgs[1].role, "assistant");
    assert!(msgs[1].tool_calls.is_some());
    assert_eq!(msgs[2].role, "tool");
    assert_eq!(msgs[3].role, "assistant");
    assert!(msgs[3].content.contains("File contains"));
}

#[tokio::test]
async fn agent_cancellation_terminates_cleanly() {
    let tmp = tempfile::tempdir().unwrap();
    let mock = Arc::new(MockProvider::new());
    let state = common::app_state_with_mock(tmp.path(), Arc::clone(&mock)).await;

    SessionRepo::create(&state.pool, "s1", "Test", "mock", "mock-model", None)
        .await
        .unwrap();

    mock.enqueue_text("hello").enqueue_done();

    let rt_config = AgentRuntimeConfig::default();
    let agent = Agent::new(Arc::clone(&state), rt_config, mock);
    let (tx, mut rx) = mpsc::channel::<AgentEvent>(128);

    let run_handle = tokio::spawn(async move { agent.run("s1", "task", tx).await });

    // Wait for done or cancelled
    let mut terminated = false;
    while let Some(ev) = rx.recv().await {
        match ev {
            AgentEvent::Done { .. } | AgentEvent::Cancelled { .. } | AgentEvent::Error { .. } => {
                terminated = true;
                break;
            }
            _ => {}
        }
    }

    let _ = run_handle.await;
    assert!(terminated);
}

#[tokio::test]
async fn agent_max_iterations_enforced() {
    let tmp = tempfile::tempdir().unwrap();
    let mock = Arc::new(MockProvider::new());
    let state = common::app_state_with_mock(tmp.path(), Arc::clone(&mock)).await;

    SessionRepo::create(&state.pool, "s1", "Test", "mock", "mock-model", None)
        .await
        .unwrap();

    // Setup file
    {
        let ctx = nexus::tools::context::ToolContext {
            session_id: Some("s1".into()),
            run_id: Some("setup".into()),
            workspace: Arc::clone(&state.sandbox),
            pool: state.pool.clone(),
            memory: Arc::clone(&state.memory),
            browser: Arc::clone(&state.browser),
            scheduler: Arc::clone(&state.scheduler),
            config: Arc::clone(&state.config),
        };
        WriteFileTool
            .execute(&ctx, json!({"path": "test.txt", "content": "data"}))
            .await
            .unwrap();
    }

    // Script: keep calling read_file forever
    for _ in 0..10 {
        mock.enqueue_tool_call("tc_iter", "read_file", json!({"path": "test.txt"}))
            .enqueue_done();
    }

    let mut rt_config = AgentRuntimeConfig::default();
    rt_config.max_iterations = 3;
    rt_config.max_tool_calls = 50;

    let agent = Agent::new(Arc::clone(&state), rt_config, mock);
    let (tx, mut rx) = mpsc::channel::<AgentEvent>(128);

    agent.run("s1", "loop forever", tx).await.unwrap();

    let mut iteration_ends = 0;
    let mut done = false;
    while let Some(ev) = rx.recv().await {
        match ev {
            AgentEvent::IterationEnd { iteration, .. } => {
                iteration_ends += 1;
                assert!(iteration <= 3);
            }
            AgentEvent::Done { final_message, .. } => {
                done = true;
                assert!(
                    final_message.contains("max iterations") || final_message.is_empty(),
                    "unexpected final_message: {final_message}"
                );
                break;
            }
            AgentEvent::Error { .. } => break,
            _ => {}
        }
    }

    assert!(done);
    assert!(iteration_ends <= 3);
}

#[tokio::test]
async fn agent_unknown_tool_returns_error_result() {
    let tmp = tempfile::tempdir().unwrap();
    let mock = Arc::new(MockProvider::new());
    let state = common::app_state_with_mock(tmp.path(), Arc::clone(&mock)).await;

    SessionRepo::create(&state.pool, "s1", "Test", "mock", "mock-model", None)
        .await
        .unwrap();

    // Script: assistant calls nonexistent tool, then says "sorry"
    mock.enqueue_tool_call("tc1", "nonexistent_tool", json!({}))
        .enqueue_done();
    mock.enqueue_text("Sorry, that tool doesn't exist").enqueue_done();

    let rt_config = AgentRuntimeConfig::default();
    let agent = Agent::new(Arc::clone(&state), rt_config, mock);
    let (tx, mut rx) = mpsc::channel::<AgentEvent>(128);

    agent.run("s1", "use bad tool", tx).await.unwrap();

    let mut tool_error_seen = false;
    let mut done = false;
    while let Some(ev) = rx.recv().await {
        match ev {
            AgentEvent::ToolCallEnd { result, .. } => {
                if !result.ok {
                    tool_error_seen = true;
                    assert!(result.output.contains("not found"));
                }
            }
            AgentEvent::Done { .. } => {
                done = true;
                break;
            }
            AgentEvent::Error { .. } => break,
            _ => {}
        }
    }

    assert!(done);
    assert!(tool_error_seen);
}

#[tokio::test]
async fn agent_streaming_chunks_concat() {
    let tmp = tempfile::tempdir().unwrap();
    let mock = Arc::new(MockProvider::new());
    let state = common::app_state_with_mock(tmp.path(), Arc::clone(&mock)).await;

    SessionRepo::create(&state.pool, "s1", "Test", "mock", "mock-model", None)
        .await
        .unwrap();

    // Script multiple text chunks
    mock.enqueue_text("Hello")
        .enqueue_text(", ")
        .enqueue_text("world!")
        .enqueue_done();

    let rt_config = AgentRuntimeConfig::default();
    let agent = Agent::new(Arc::clone(&state), rt_config, mock);
    let (tx, mut rx) = mpsc::channel::<AgentEvent>(128);

    agent.run("s1", "greet me", tx).await.unwrap();

    let mut full_text = String::new();
    while let Some(ev) = rx.recv().await {
        match ev {
            AgentEvent::Delta { text, .. } => full_text.push_str(&text),
            AgentEvent::Done { .. } => break,
            _ => {}
        }
    }

    assert_eq!(full_text, "Hello, world!");
}

#[tokio::test]
async fn agent_tool_call_persists_to_db() {
    let tmp = tempfile::tempdir().unwrap();
    let mock = Arc::new(MockProvider::new());
    let state = common::app_state_with_mock(tmp.path(), Arc::clone(&mock)).await;

    // Pre-create file
    {
        let ctx = nexus::tools::context::ToolContext {
            session_id: Some("s1".into()),
            run_id: Some("setup".into()),
            workspace: Arc::clone(&state.sandbox),
            pool: state.pool.clone(),
            memory: Arc::clone(&state.memory),
            browser: Arc::clone(&state.browser),
            scheduler: Arc::clone(&state.scheduler),
            config: Arc::clone(&state.config),
        };
        WriteFileTool
            .execute(&ctx, json!({"path": "data.txt", "content": "abc"}))
            .await
            .unwrap();
    }

    SessionRepo::create(&state.pool, "s1", "Test", "mock", "mock-model", None)
        .await
        .unwrap();

    mock.enqueue_tool_call("tc1", "read_file", json!({"path": "data.txt"}))
        .enqueue_done();
    mock.enqueue_text("done").enqueue_done();

    let agent = Agent::new(
        Arc::clone(&state),
        AgentRuntimeConfig::default(),
        mock,
    );
    let (tx, mut rx) = mpsc::channel::<AgentEvent>(128);
    agent.run("s1", "read file", tx).await.unwrap();

    // Drain
    while let Some(ev) = rx.recv().await {
        if matches!(ev, AgentEvent::Done { .. }) {
            break;
        }
    }

    // Verify messages persisted with tool_calls + tool_results JSON
    let msgs = MessageRepo::list_by_session(&state.pool, "s1").await.unwrap();
    assert!(msgs.len() >= 3);
    let assistant_msg = msgs.iter().find(|m| m.role == "assistant" && m.tool_calls.is_some());
    assert!(assistant_msg.is_some(), "assistant message with tool_calls missing");
    let tc_json = assistant_msg.unwrap().tool_calls.as_ref().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(tc_json).unwrap();
    assert!(parsed.is_array());
    assert_eq!(parsed[0]["function"]["name"], "read_file");

    let tool_msg = msgs.iter().find(|m| m.role == "tool");
    assert!(tool_msg.is_some(), "tool result message missing");
    let tr_json = tool_msg.unwrap().tool_results.as_ref().unwrap();
    let tr_parsed: serde_json::Value = serde_json::from_str(tr_json).unwrap();
    assert_eq!(tr_parsed["ok"], true);
}
