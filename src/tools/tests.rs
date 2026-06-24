//! Tool registry + schema tests.

#![cfg(test)]

use super::registry::ToolRegistry;
use super::schema::ToolSchema;
use super::tool::{Tool, ToolResult};
use crate::error::Result;
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use async_trait::async_trait;
use serde_json::json;

struct DummyTool {
    name: &'static str,
    perm: PermissionLevel,
}

#[async_trait]
impl Tool for DummyTool {
    fn name(&self) -> &'static str { self.name }
    fn description(&self) -> &'static str { "dummy" }
    fn permission(&self) -> PermissionLevel { self.perm }
    fn schema(&self) -> serde_json::Value {
        json!({"type": "object", "properties": {}})
    }
    async fn execute(&self, _ctx: &ToolContext, _input: serde_json::Value) -> Result<ToolResult> {
        Ok(ToolResult::ok("", self.name, "ok"))
    }
}

#[test]
fn registry_register_and_get() {
    let reg = ToolRegistry::new();
    reg.register(DummyTool { name: "tool_a", perm: PermissionLevel::Safe });
    reg.register(DummyTool { name: "tool_b", perm: PermissionLevel::RequiresApproval });

    assert_eq!(reg.len(), 2);
    assert!(reg.get("tool_a").is_some());
    assert!(reg.get("tool_b").is_some());
    assert!(reg.get("nonexistent").is_none());
}

#[test]
fn registry_list_names() {
    let reg = ToolRegistry::new();
    reg.register(DummyTool { name: "alpha", perm: PermissionLevel::Safe });
    reg.register(DummyTool { name: "beta", perm: PermissionLevel::Safe });

    let mut names = reg.list_names();
    names.sort();
    assert_eq!(names, vec!["alpha".to_string(), "beta".to_string()]);
}

#[test]
fn registry_all_schemas() {
    let reg = ToolRegistry::new();
    reg.register(DummyTool { name: "x", perm: PermissionLevel::Safe });
    let schemas = reg.all_schemas();
    assert_eq!(schemas.len(), 1);
    assert_eq!(schemas[0].name, "x");
    assert_eq!(schemas[0].description, "dummy");
}

#[test]
fn registry_overwrite_on_same_name() {
    let reg = ToolRegistry::new();
    reg.register(DummyTool { name: "tool", perm: PermissionLevel::Safe });
    reg.register(DummyTool { name: "tool", perm: PermissionLevel::Dangerous });
    assert_eq!(reg.len(), 1);
    let t = reg.get("tool").unwrap();
    assert_eq!(t.permission(), PermissionLevel::Dangerous);
}

#[test]
fn schema_object_helper() {
    let s = ToolSchema::object(
        "test_tool",
        "a test tool",
        vec![
            ("path", "string", "File path", true),
            ("content", "string", "File content", true),
            ("verbose", "boolean", "Verbose flag", false),
        ],
    );
    assert_eq!(s.name, "test_tool");
    let props = s.parameters.get("properties").unwrap().as_object().unwrap();
    assert_eq!(props.len(), 3);
    let required = s.parameters.get("required").unwrap().as_array().unwrap();
    assert_eq!(required.len(), 2);
}

#[test]
fn tool_result_ok_helper() {
    let r = ToolResult::ok("c1", "read_file", "hello");
    assert!(r.ok);
    assert_eq!(r.call_id, "c1");
    assert_eq!(r.tool, "read_file");
    assert_eq!(r.output, "hello");
    assert!(r.data.is_none());
}

#[test]
fn tool_result_error_helper() {
    let r = ToolResult::error("c2", "delete_file", "permission denied");
    assert!(!r.ok);
    assert_eq!(r.output, "permission denied");
}

#[test]
fn tool_result_with_data_and_duration() {
    let r = ToolResult::ok("c3", "list_dir", "1 entry")
        .with_data(json!({"count": 1}))
        .with_duration(42);
    assert_eq!(r.data, Some(json!({"count": 1})));
    assert_eq!(r.duration_ms, 42);
}
