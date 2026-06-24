//! IPC contract tests — verify Tauri command signatures + payloads.
//!
//! These tests don't spin up a Tauri app — they verify that the command
//! functions exist with correct signatures and that the IpcError type
//! serializes correctly.

#![cfg(feature = "test-utils")]

mod common;

use nexus::commands::{IpcError, AppStateRef};
use nexus::error::NexusError;
use serde_json::json;
use std::sync::Arc;

// === IpcError serialization ===

#[test]
fn ipc_error_from_database_error() {
    let err = NexusError::Database(sqlx::Error::PoolClosed);
    let ipc: IpcError = err.into();
    assert_eq!(ipc.code, "database");
    assert!(!ipc.message.is_empty());
}

#[test]
fn ipc_error_from_llm_error() {
    let err = NexusError::Llm(nexus::error::LlmError::InvalidApiKey {
        provider: "openai".into(),
    });
    let ipc: IpcError = err.into();
    assert_eq!(ipc.code, "llm");
    assert!(ipc.message.contains("openai"));
}

#[test]
fn ipc_error_from_security_error() {
    let err = NexusError::Security(nexus::error::SecurityError::SandboxViolation(
        "/etc/passwd".into(),
    ));
    let ipc: IpcError = err.into();
    assert_eq!(ipc.code, "security");
    assert!(ipc.message.contains("/etc/passwd"));
}

#[test]
fn ipc_error_from_cancelled() {
    let err = NexusError::Cancelled;
    let ipc: IpcError = err.into();
    assert_eq!(ipc.code, "cancelled");
}

#[test]
fn ipc_error_from_not_found() {
    let err = NexusError::NotFound("session xyz".into());
    let ipc: IpcError = err.into();
    assert_eq!(ipc.code, "not_found");
}

#[test]
fn ipc_error_serializes_to_json() {
    let ipc = IpcError {
        code: "test_code".into(),
        message: "test message".into(),
    };
    let json = serde_json::to_string(&ipc).unwrap();
    assert!(json.contains("test_code"));
    assert!(json.contains("test message"));
}

// === Command signature presence (compile-time check) ===

#[test]
fn chat_commands_exist() {
    // Just reference the function — compile-time check
    let _ = nexus::commands::chat::chat_send as fn(_, _, _) -> _;
    let _ = nexus::commands::chat::chat_cancel as fn(_, _) -> _;
}

#[test]
fn session_commands_exist() {
    let _ = nexus::commands::session::session_create as fn(_, _) -> _;
    let _ = nexus::commands::session::session_list as fn(_, _) -> _;
    let _ = nexus::commands::session::session_search as fn(_, _, _) -> _;
    let _ = nexus::commands::session::session_rename as fn(_, _, _) -> _;
    let _ = nexus::commands::session::session_delete as fn(_, _) -> _;
}

#[test]
fn memory_commands_exist() {
    let _ = nexus::commands::memory::memory_save as fn(_, _) -> _;
    let _ = nexus::commands::memory::memory_recall as fn(_, _) -> _;
    let _ = nexus::commands::memory::memory_list as fn(_, _) -> _;
    let _ = nexus::commands::memory::memory_delete as fn(_, _) -> _;
}

#[test]
fn scheduler_commands_exist() {
    let _ = nexus::commands::scheduler::scheduler_add as fn(_, _) -> _;
    let _ = nexus::commands::scheduler::scheduler_list as fn(_) -> _;
    let _ = nexus::commands::scheduler::scheduler_cancel as fn(_, _) -> _;
}

#[test]
fn tool_commands_exist() {
    let _ = nexus::commands::tool::tool_list as fn(_) -> _;
    let _ = nexus::commands::tool::tool_invoke as fn(_, _) -> _;
}

#[test]
fn browser_commands_exist() {
    let _ = nexus::commands::browser::browser_action as fn(_, _) -> _;
    let _ = nexus::commands::browser::browser_shutdown as fn(_) -> _;
}

#[test]
fn config_commands_exist() {
    let _ = nexus::commands::config::config_get as fn(_) -> _;
    let _ = nexus::commands::config::config_set as fn(_, _) -> _;
}

#[test]
fn approval_commands_exist() {
    let _ = nexus::commands::approval::approval_respond as fn(_, _) -> _;
    let _ = nexus::commands::approval::approval_pending as fn(_) -> _;
}

// === DTO type checks ===

#[test]
fn session_dto_roundtrip() {
    let dto = nexus::commands::session::SessionDto {
        id: "s1".into(),
        title: "Test".into(),
        provider: "openai".into(),
        model: "gpt-4o".into(),
        system_prompt: None,
        created_at: 1234567890,
        updated_at: 1234567890,
    };
    let json = serde_json::to_string(&dto).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["id"], "s1");
    assert_eq!(parsed["title"], "Test");
    assert_eq!(parsed["provider"], "openai");
}

#[test]
fn memory_dto_roundtrip() {
    let dto = nexus::commands::memory::MemoryDto {
        id: "m1".into(),
        content: "test fact".into(),
        category: "fact".into(),
        tags: vec!["test".into()],
        session_id: Some("s1".into()),
        created_at: 1234567890,
        last_used_at: 1234567890,
        use_count: 3,
    };
    let json = serde_json::to_string(&dto).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["category"], "fact");
    assert_eq!(parsed["use_count"], 3);
}

#[test]
fn scheduler_dto_roundtrip() {
    let dto = nexus::commands::scheduler::SchedulerJobDto {
        id: "j1".into(),
        kind: "recurring 0 9 * * *".into(),
        message: "morning".into(),
        session_id: None,
        enabled: true,
        created_at: 1234567890,
    };
    let json = serde_json::to_string(&dto).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["enabled"], true);
}

#[test]
fn tool_info_dto_roundtrip() {
    let dto = nexus::commands::tool::ToolInfoDto {
        name: "read_file".into(),
        description: "Read a file".into(),
        permission: "safe".into(),
        schema: json!({"type": "object"}),
    };
    let json = serde_json::to_string(&dto).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["name"], "read_file");
    assert_eq!(parsed["permission"], "safe");
}

// === Approval input validation ===

#[test]
fn approval_respond_input_parses_approved() {
    let json_str = r#"{"request_id": "req1", "decision": "approved"}"#;
    let parsed: nexus::commands::approval::ApprovalRespondInput =
        serde_json::from_str(json_str).unwrap();
    assert_eq!(parsed.request_id, "req1");
    assert_eq!(parsed.decision, "approved");
}

#[test]
fn approval_respond_input_parses_rejected() {
    let json_str = r#"{"request_id": "req2", "decision": "rejected"}"#;
    let parsed: nexus::commands::approval::ApprovalRespondInput =
        serde_json::from_str(json_str).unwrap();
    assert_eq!(parsed.decision, "rejected");
}

// === AppState integration with commands (smoke test) ===

#[tokio::test]
async fn app_state_has_all_stores() {
    let tmp = tempfile::tempdir().unwrap();
    let mock = Arc::new(nexus::llm::MockProvider::new());
    let state = common::app_state_with_mock(tmp.path(), mock).await;

    // Verify all stores accessible
    assert!(!state.tool_registry.is_empty());
    assert!(state.tool_registry.len() >= 24); // 24 default tools
    let _ = &state.memory;
    let _ = &state.scheduler;
    let _ = &state.browser;
    let _ = &state.sandbox;
    let _ = &state.approval_gate;
    let _ = &state.config;

    // Use AppStateRef type to ensure it compiles
    let _state_ref_type: Option<AppStateRef<'_>> = None;
}

#[tokio::test]
async fn tool_registry_lists_all_tools_via_commands_layer() {
    let tmp = tempfile::tempdir().unwrap();
    let mock = Arc::new(nexus::llm::MockProvider::new());
    let state = common::app_state_with_mock(tmp.path(), mock).await;

    let schemas = state.tool_registry.all_schemas();
    let names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();

    // Verify all 24 tool names
    let expected = [
        "read_file",
        "write_file",
        "append_file",
        "delete_file",
        "move_file",
        "copy_file",
        "list_directory",
        "search_files",
        "create_directory",
        "run_command",
        "run_python",
        "run_javascript",
        "browser_navigate",
        "browser_click",
        "browser_type",
        "browser_wait",
        "browser_extract_text",
        "browser_screenshot",
        "web_search",
        "memory_save",
        "memory_recall",
        "memory_delete",
        "schedule_one_time",
        "schedule_recurring",
        "list_scheduled",
        "cancel_scheduled",
    ];

    for name in &expected {
        assert!(
            names.contains(name),
            "tool `{name}` missing from registry (have: {:?})",
            names
        );
    }
}
