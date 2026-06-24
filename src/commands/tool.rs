//! Tool IPC commands (manual mode — direct tool invocation).

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

use crate::security::approval::{ApprovalDecision, ApprovalGate, ApprovalRequest};
use crate::state::AppState;
use crate::tools::context::ToolContext;
use crate::tools::schema::ToolSchema;

use super::{IpcError, IpcResult};

#[derive(Debug, Serialize)]
pub struct ToolInfoDto {
    pub name: String,
    pub description: String,
    pub permission: String,
    pub schema: serde_json::Value,
}

#[tauri::command]
pub async fn tool_list(state: State<'_, Arc<AppState>>) -> IpcResult<Vec<ToolInfoDto>> {
    let schemas: Vec<ToolSchema> = state.tool_registry.all_schemas();
    let mut out = Vec::with_capacity(schemas.len());
    for s in schemas {
        if let Some(tool) = state.tool_registry.get(&s.name) {
            out.push(ToolInfoDto {
                name: s.name,
                description: s.description,
                permission: tool.permission().label().to_string(),
                schema: s.parameters,
            });
        }
    }
    Ok(out)
}

#[derive(Debug, Deserialize)]
pub struct ToolInvokeInput {
    pub name: String,
    pub input: serde_json::Value,
    pub session_id: Option<String>,
}

#[tauri::command]
pub async fn tool_invoke(
    state: State<'_, Arc<AppState>>,
    input: ToolInvokeInput,
) -> IpcResult<serde_json::Value> {
    let tool = state
        .tool_registry
        .get(&input.name)
        .ok_or_else(|| IpcError {
            code: "not_found".into(),
            message: format!("tool `{}` not found", input.name),
        })?;

    // Approval flow cho non-safe tools
    let permission = tool.permission();
    if permission.requires_approval() {
        let req_id = ApprovalGate::new_id();
        let req = ApprovalRequest {
            id: req_id.clone(),
            tool: input.name.clone(),
            input: input.input.clone(),
            permission,
            session_id: input.session_id.clone(),
            run_id: "manual".into(),
        };
        // Manual mode — caller frontend phải handle approval event + call approval_respond.
        let decision = state
            .approval_gate
            .request(req, |_r| {})
            .await
            .map_err(IpcError::from)?;
        if decision == ApprovalDecision::Rejected {
            return Err(IpcError {
                code: "rejected".into(),
                message: "user rejected tool call".into(),
            });
        }
    }

    let ctx = ToolContext {
        session_id: input.session_id.clone(),
        run_id: Some("manual".into()),
        workspace: Arc::clone(&state.sandbox),
        pool: state.pool.clone(),
        memory: Arc::clone(&state.memory),
        browser: Arc::clone(&state.browser),
        scheduler: Arc::clone(&state.scheduler),
        config: Arc::clone(&state.config),
    };

    let result = tool.execute(&ctx, input.input).await.map_err(IpcError::from)?;
    Ok(serde_json::to_value(&result).map_err(|e| IpcError {
        code: "serde".into(),
        message: e.to_string(),
    })?)
}
