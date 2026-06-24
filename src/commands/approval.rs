//! Approval IPC commands.

use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

use crate::security::approval::ApprovalDecision;
use crate::state::AppState;

use super::{IpcError, IpcResult};

#[derive(Debug, Deserialize)]
pub struct ApprovalRespondInput {
    pub request_id: String,
    pub decision: String,                     // "approved" | "rejected"
}

#[tauri::command]
pub async fn approval_respond(
    state: State<'_, Arc<AppState>>,
    input: ApprovalRespondInput,
) -> IpcResult<()> {
    let decision = match input.decision.as_str() {
        "approved" | "approve" | "ok" | "yes" => ApprovalDecision::Approved,
        "rejected" | "reject" | "no" | "cancel" => ApprovalDecision::Rejected,
        other => {
            return Err(IpcError {
                code: "invalid_argument".into(),
                message: format!("unknown decision: {other}"),
            });
        }
    };
    state
        .approval_gate
        .respond(&input.request_id, decision)
        .await
        .map_err(IpcError::from)?;
    Ok(())
}

#[tauri::command]
pub async fn approval_pending(state: State<'_, Arc<AppState>>) -> IpcResult<Vec<serde_json::Value>> {
    let pending = state.approval_gate.pending().await;
    Ok(pending
        .into_iter()
        .map(|p| serde_json::to_value(&p).unwrap_or(serde_json::Value::Null))
        .collect())
}
