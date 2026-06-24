//! run_command tool.

use async_trait::async_trait;
use serde_json::json;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

use crate::database::repositories::command_log_repo::{CommandLogRepo, CommandLogRow};
use crate::error::{NexusError, Result};
use crate::security::permission::PermissionLevel;
use crate::security::blacklist::CommandBlacklist;
use crate::tools::context::ToolContext;
use crate::tools::tool::{Tool, ToolResult};
use crate::utils::ids::new_uuid;
use crate::utils::time::now_ts;
use crate::utils::truncate::{truncate_kb, format_duration};

pub struct RunCommandTool {
    blacklist: CommandBlacklist,
}

impl RunCommandTool {
    #[must_use]
    pub fn new() -> Self {
        Self {
            blacklist: CommandBlacklist::new(),
        }
    }
}

impl Default for RunCommandTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for RunCommandTool {
    fn name(&self) -> &'static str { "run_command" }

    fn description(&self) -> &'static str {
        "Execute a shell command in the workspace sandbox. Subject to blacklist + approval + timeout + output truncation."
    }

    fn permission(&self) -> PermissionLevel { PermissionLevel::RequiresApproval }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Shell command to execute." },
                "args": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Arguments (passed separately to avoid shell injection).",
                    "default": []
                },
                "timeout_secs": { "type": "integer", "default": 60, "maximum": 600 },
                "cwd": { "type": "string", "description": "Working directory (inside sandbox). Default: workspace root." }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> Result<ToolResult> {
        let command = input.get("command").and_then(|v| v.as_str())
            .ok_or_else(|| NexusError::InvalidArgument("missing `command`".into()))?;
        let args: Vec<String> = input.get("args").and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let timeout_secs = input.get("timeout_secs").and_then(|v| v.as_u64())
            .unwrap_or(ctx.config.security.shell_timeout_secs)
            .min(600);
        let cwd_str = input.get("cwd").and_then(|v| v.as_str()).unwrap_or("");

        // Blacklist check
        if let Some(reason) = self.blacklist.check(command) {
            self.log_command(ctx, command, &args, "blacklisted", None, None, None).await?;
            return Ok(ToolResult::error("", self.name(), format!("command blacklisted: {reason}")));
        }

        // Resolve cwd
        let cwd = if cwd_str.is_empty() {
            ctx.workspace.root().to_path_buf()
        } else {
            ctx.workspace.resolve(cwd_str)?
        };

        // Build command
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(command);
            c.args(&args);
            c
        } else {
            let mut c = Command::new("sh");
            c.arg("-c").arg(command);
            c
        };
        cmd.current_dir(&cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let started = std::time::Instant::now();
        let log_id = new_uuid();

        // Insert initial log row
        let log_row = CommandLogRow {
            id: log_id.clone(),
            session_id: ctx.session_id.clone(),
            command: command.to_string(),
            args: json!(args).to_string(),
            status: "executed".into(),
            exit_code: None,
            stdout: None,
            stderr: None,
            started_at: now_ts(),
            finished_at: None,
        };
        CommandLogRepo::insert(&ctx.pool, &log_row).await?;

        // Spawn
        let child = cmd.spawn()?;
        let _pid = child.id();

        // wait_with_output với timeout
        let wait_fut = child.wait_with_output();
        let output = match timeout(Duration::from_secs(timeout_secs), wait_fut).await {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => {
                CommandLogRepo::update_result(&ctx.pool, &log_id, "error", None, None, Some(&e.to_string())).await?;
                return Ok(ToolResult::error("", self.name(), format!("spawn error: {e}")));
            }
            Err(_) => {
                // Kill child on timeout
                // child moved into wait_with_output
                CommandLogRepo::update_result(&ctx.pool, &log_id, "timeout", None, None, Some("timeout")).await?;
                return Ok(ToolResult::error("", self.name(), format!("timeout after {}", format_duration(Duration::from_secs(timeout_secs)))));
            }
        };

        let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code();

        // Truncate
        let max_kb = ctx.config.security.shell_max_output_kb;
        let stdout_trunc = truncate_kb(&stdout_str, max_kb);
        let stderr_trunc = truncate_kb(&stderr_str, max_kb);
        let stdout_final = stdout_trunc.content;
        let stderr_final = stderr_trunc.content;

        // Update log
        let status_str = if output.status.success() { "executed" } else { "error" };
        CommandLogRepo::update_result(
            &ctx.pool,
            &log_id,
            status_str,
            exit_code.map(|c| c as i64),
            Some(&stdout_final),
            Some(&stderr_final),
        ).await?;

        let elapsed = started.elapsed();
        let output_text = format!(
            "exit: {}\nduration: {}\nstdout:\n{}\nstderr:\n{}",
            exit_code.map_or("N/A".into(), |c| c.to_string()),
            format_duration(elapsed),
            stdout_final,
            stderr_final,
        );

        if output.status.success() {
            Ok(ToolResult::ok("", self.name(), output_text)
                .with_data(json!({
                    "exit_code": exit_code,
                    "duration_ms": elapsed.as_millis() as u64,
                    "stdout_truncated": stdout_trunc.truncated,
                    "stderr_truncated": stderr_trunc.truncated,
                })))
        } else {
            Ok(ToolResult::error("", self.name(), output_text))
        }
    }
}

impl RunCommandTool {
    async fn log_command(
        &self,
        ctx: &ToolContext,
        command: &str,
        args: &[String],
        status: &str,
        exit_code: Option<i64>,
        stdout: Option<&str>,
        stderr: Option<&str>,
    ) -> Result<()> {
        let row = CommandLogRow {
            id: new_uuid(),
            session_id: ctx.session_id.clone(),
            command: command.to_string(),
            args: json!(args).to_string(),
            status: status.to_string(),
            exit_code.map(|c| c as i64),
            stdout: stdout.map(String::from),
            stderr: stderr.map(String::from),
            started_at: now_ts(),
            finished_at: Some(now_ts()),
        };
        CommandLogRepo::insert(&ctx.pool, &row).await?;
        Ok(())
    }
}
