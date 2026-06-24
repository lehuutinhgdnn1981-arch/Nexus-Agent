//! run_python tool — execute Python code in workspace sandbox.

use async_trait::async_trait;
use serde_json::json;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

use crate::error::{NexusError, Result};
use crate::security::permission::PermissionLevel;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};
use crate::utils::truncate::{format_duration, truncate_kb};

pub struct RunPythonTool;

#[async_trait]
impl Tool for RunPythonTool {
    fn name(&self) -> &'static str { "run_python" }

    fn description(&self) -> &'static str {
        "Execute Python code in an isolated workspace. Requires Python 3 installed on PATH."
    }

    fn permission(&self) -> PermissionLevel { PermissionLevel::RequiresApproval }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "code": { "type": "string", "description": "Python code to execute." },
                "timeout_secs": { "type": "integer", "default": 30, "maximum": 120 }
            },
            "required": ["code"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> Result<ToolResult> {
        let code = input.get("code").and_then(|v| v.as_str())
            .ok_or_else(|| NexusError::InvalidArgument("missing `code`".into()))?;
        let timeout_secs = input.get("timeout_secs").and_then(|v| v.as_u64()).unwrap_or(30).min(120);

        let python = which::which("python3").or_else(|_| which::which("python"))
            .map_err(|_| NexusError::NotFound("python3 not found on PATH".into()))?;

        // Write code to temp file in workspace
        let script_path = ctx.workspace.resolve_with_parents(&format!(".nexus_scripts/{}.py", crate::utils::ids::short_id()))?;
        std::fs::write(&script_path, code)?;

        let mut cmd = Command::new(python);
        cmd.arg(&script_path)
            .current_dir(ctx.workspace.root())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let started = std::time::Instant::now();
        let output = match timeout(Duration::from_secs(timeout_secs), cmd.output()).await {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => return Ok(ToolResult::error("", self.name(), format!("spawn: {e}"))),
            Err(_) => return Ok(ToolResult::error("", self.name(), format!("timeout after {}", format_duration(Duration::from_secs(timeout_secs))))),
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code();
        let max_kb = ctx.config.security.shell_max_output_kb;
        let stdout_t = truncate_kb(&stdout, max_kb);
        let stderr_t = truncate_kb(&stderr, max_kb);

        // Cleanup script
        let _ = std::fs::remove_file(&script_path);

        let text = format!(
            "exit: {}\nduration: {}\nstdout:\n{}\nstderr:\n{}",
            exit_code.map_or("N/A".into(), |c| c.to_string()),
            format_duration(started.elapsed()),
            stdout_t.content,
            stderr_t.content,
        );

        if output.status.success() {
            Ok(ToolResult::ok("", self.name(), text).with_data(json!({
                "exit_code": exit_code,
                "duration_ms": started.elapsed().as_millis() as u64,
            })))
        } else {
            Ok(ToolResult::error("", self.name(), text))
        }
    }
}
