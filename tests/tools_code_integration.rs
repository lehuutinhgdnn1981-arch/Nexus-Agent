//! Integration tests cho code execution tools.

mod common;

use nexus::tools::code::{run_javascript::RunJavaScriptTool, run_python::RunPythonTool};
use nexus::tools::tool::Tool;
use serde_json::json;

#[tokio::test]
async fn integration_python_basic_io() {
    if which::which("python3").is_err() && which::which("python").is_err() {
        eprintln!("python3 not installed; skipping");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let ctx = common::tool_context(tmp.path()).await;

    let tool = RunPythonTool;
    let code = r#"
import sys
print("stdout_test")
print("stderr_test", file=sys.stderr)
print("final")
"#;
    let r = tool.execute(&ctx, json!({"code": code})).await.unwrap();
    assert!(r.ok);
    assert!(r.output.contains("stdout_test"));
    assert!(r.output.contains("stderr_test"));
    assert!(r.output.contains("final"));
}

#[tokio::test]
async fn integration_python_arithmetic() {
    if which::which("python3").is_err() && which::which("python").is_err() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let ctx = common::tool_context(tmp.path()).await;

    let tool = RunPythonTool;
    let r = tool
        .execute(&ctx, json!({"code": "print(2 + 3 * 4)"}))
        .await
        .unwrap();
    assert!(r.ok);
    assert!(r.output.contains("14"));
}

#[tokio::test]
async fn integration_python_can_read_workspace_files() {
    if which::which("python3").is_err() && which::which("python").is_err() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let ctx = common::tool_context(tmp.path()).await;

    // Setup: write a file via write_file tool, then read it from python
    use nexus::tools::file::write_file::WriteFileTool;
    WriteFileTool
        .execute(&ctx, json!({"path": "data.txt", "content": "42"}))
        .await
        .unwrap();

    let tool = RunPythonTool;
    let code = r#"
with open('data.txt') as f:
    n = int(f.read().strip())
print(n * 2)
"#;
    let r = tool.execute(&ctx, json!({"code": code})).await.unwrap();
    assert!(r.ok);
    assert!(r.output.contains("84"));
}

#[tokio::test]
async fn integration_javascript_basic() {
    if which::which("node").is_err() {
        eprintln!("node not installed; skipping");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let ctx = common::tool_context(tmp.path()).await;

    let tool = RunJavaScriptTool;
    let r = tool
        .execute(&ctx, json!({"code": "console.log(2 + 3 * 4)"}))
        .await
        .unwrap();
    assert!(r.ok);
    assert!(r.output.contains("14"));
}

#[tokio::test]
async fn integration_javascript_error_propagates() {
    if which::which("node").is_err() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let ctx = common::tool_context(tmp.path()).await;

    let tool = RunJavaScriptTool;
    let r = tool
        .execute(&ctx, json!({"code": "throw new Error('boom_int')"}))
        .await
        .unwrap();
    assert!(!r.ok);
    assert!(r.output.contains("boom_int"));
}
