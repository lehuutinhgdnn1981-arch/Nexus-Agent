//! Integration tests cho shell tool.

mod common;

use nexus::database::repositories::command_log_repo::CommandLogRepo;
use nexus::tools::shell::run_command::RunCommandTool;
use nexus::tools::tool::Tool;
use serde_json::json;

#[tokio::test]
async fn integration_shell_echo() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = common::tool_context(tmp.path()).await;

    let tool = RunCommandTool::new();
    let r = tool
        .execute(&ctx, json!({"command": "echo integration_test_ok"}))
        .await
        .unwrap();
    assert!(r.ok);
    assert!(r.output.contains("integration_test_ok"));
}

#[tokio::test]
async fn integration_shell_pipes() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = common::tool_context(tmp.path()).await;

    let tool = RunCommandTool::new();
    #[cfg(unix)]
    let cmd = "echo 'a\\nb\\nc' | grep b";
    #[cfg(windows)]
    let cmd = "echo a & echo b & echo c";

    let r = tool.execute(&ctx, json!({"command": cmd})).await.unwrap();
    assert!(r.ok);
}

#[tokio::test]
async fn integration_shell_cwd_default_workspace() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = common::tool_context(tmp.path()).await;

    let tool = RunCommandTool::new();
    #[cfg(unix)]
    let cmd = "pwd";
    #[cfg(windows)]
    let cmd = "cd";

    let r = tool.execute(&ctx, json!({"command": cmd})).await.unwrap();
    assert!(r.ok);
    let workspace_name = tmp.path().file_name().unwrap().to_str().unwrap();
    assert!(r.output.contains(workspace_name));
}

#[tokio::test]
async fn integration_shell_blacklist_persists_log() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = common::tool_context(tmp.path()).await;

    let tool = RunCommandTool::new();
    let r = tool
        .execute(&ctx, json!({"command": "rm -rf /"}))
        .await
        .unwrap();
    assert!(!r.ok);

    let logs = CommandLogRepo::list_recent(&ctx.pool, 10).await.unwrap();
    let blacklisted = logs.iter().find(|l| l.command.contains("rm -rf"));
    assert!(blacklisted.is_some());
    assert_eq!(blacklisted.unwrap().status, "blacklisted");
}

#[tokio::test]
async fn integration_shell_exit_code_propagated() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = common::tool_context(tmp.path()).await;

    let tool = RunCommandTool::new();
    #[cfg(unix)]
    let cmd = "sh -c 'exit 7'";
    #[cfg(windows)]
    let cmd = "exit /b 7";

    let r = tool.execute(&ctx, json!({"command": cmd})).await.unwrap();
    assert!(!r.ok);
    assert!(r.output.contains("7"));
}

#[tokio::test]
async fn integration_shell_timeout_works() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = common::tool_context(tmp.path()).await;

    let tool = RunCommandTool::new();
    #[cfg(unix)]
    let cmd = "sleep 10";
    #[cfg(windows)]
    let cmd = "timeout /t 10 /nobreak > nul";

    let r = tool
        .execute(&ctx, json!({"command": cmd, "timeout_secs": 1}))
        .await
        .unwrap();
    assert!(!r.ok);
    assert!(r.output.contains("timeout"));
}
