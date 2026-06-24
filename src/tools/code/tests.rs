//! Code execution tools — unit tests.

#![cfg(test)]

use std::sync::Arc;

use serde_json::json;

use super::super::context::ToolContext;
use crate::browser::BrowserManager;
use crate::config::AppConfig;
use crate::database::pool::in_memory_pool;
use crate::llm::factory::build_provider;
use crate::memory::embedding::EmbeddingClient;
use crate::memory::store::MemoryStore;
use crate::scheduler::SchedulerService;
use crate::security::Sandbox;
use crate::tools::code::{run_javascript::RunJavaScriptTool, run_python::RunPythonTool};
use crate::tools::tool::Tool;

async fn make_ctx(workspace_root: std::path::PathBuf) -> ToolContext {
    let pool = in_memory_pool().await.unwrap();
    let config = Arc::new(AppConfig::defaults());
    let sandbox = Arc::new(Sandbox::new(workspace_root));
    let provider = build_provider("ollama", &config.llm.ollama, &config.memory.embedding_model).unwrap();
    let embedding_client = Arc::new(EmbeddingClient::new(provider));
    let memory = Arc::new(MemoryStore::new(pool.clone(), embedding_client, 0.92));
    let browser = Arc::new(BrowserManager::new(true, 9222));
    let on_fire: crate::scheduler::service::FireCallback = Arc::new(|_, _, _| {});
    let scheduler = Arc::new(SchedulerService::new(pool.clone(), on_fire));

    ToolContext {
        session_id: Some("test".into()),
        run_id: Some("test".into()),
        workspace: sandbox,
        pool,
        memory,
        browser,
        scheduler,
        config,
    }
}

#[tokio::test]
async fn python_print() {
    // Skip if python3 not installed
    if which::which("python3").is_err() && which::which("python").is_err() {
        eprintln!("python3 not found, skipping test");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let ctx = make_ctx(tmp.path().to_path_buf()).await;

    let tool = RunPythonTool;
    let r = tool
        .execute(&ctx, json!({"code": "print('hello_from_python')"}))
        .await
        .unwrap();
    assert!(r.ok, "output: {}", r.output);
    assert!(r.output.contains("hello_from_python"));
}

#[tokio::test]
async fn python_error_captured() {
    if which::which("python3").is_err() && which::which("python").is_err() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let ctx = make_ctx(tmp.path().to_path_buf()).await;

    let tool = RunPythonTool;
    let r = tool
        .execute(&ctx, json!({"code": "raise RuntimeError('boom')"}))
        .await
        .unwrap();
    assert!(!r.ok);
    assert!(r.output.contains("RuntimeError") || r.output.contains("boom"));
}

#[tokio::test]
async fn python_exit_code_propagated() {
    if which::which("python3").is_err() && which::which("python").is_err() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let ctx = make_ctx(tmp.path().to_path_buf()).await;

    let tool = RunPythonTool;
    let r = tool
        .execute(&ctx, json!({"code": "import sys; sys.exit(42)"}))
        .await
        .unwrap();
    assert!(!r.ok);
    assert!(r.output.contains("42"));
}

#[tokio::test]
async fn javascript_print() {
    if which::which("node").is_err() {
        eprintln!("node not found, skipping test");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let ctx = make_ctx(tmp.path().to_path_buf()).await;

    let tool = RunJavaScriptTool;
    let r = tool
        .execute(&ctx, json!({"code": "console.log('hello_from_node')"}))
        .await
        .unwrap();
    assert!(r.ok, "output: {}", r.output);
    assert!(r.output.contains("hello_from_node"));
}

#[tokio::test]
async fn javascript_error_captured() {
    if which::which("node").is_err() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let ctx = make_ctx(tmp.path().to_path_buf()).await;

    let tool = RunJavaScriptTool;
    let r = tool
        .execute(&ctx, json!({"code": "throw new Error('boom_js')"}))
        .await
        .unwrap();
    assert!(!r.ok);
    assert!(r.output.contains("boom_js") || r.output.contains("Error"));
}

#[tokio::test]
async fn code_timeout() {
    if which::which("python3").is_err() && which::which("python").is_err() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let ctx = make_ctx(tmp.path().to_path_buf()).await;

    let tool = RunPythonTool;
    let r = tool
        .execute(
            &ctx,
            json!({"code": "import time; time.sleep(10)", "timeout_secs": 1}),
        )
        .await
        .unwrap();
    assert!(!r.ok);
    assert!(r.output.contains("timeout"));
}
