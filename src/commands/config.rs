//! Config IPC commands.

use std::sync::Arc;
use tauri::State;

use crate::config::store::ConfigStore;
use crate::config::AppConfig;
use crate::state::AppState;

use super::{IpcError, IpcResult};

#[tauri::command]
pub async fn config_get(state: State<'_, Arc<AppState>>) -> IpcResult<AppConfig> {
    Ok((*state.config).clone())
}

#[derive(Debug, serde::Deserialize)]
pub struct ConfigPatchInput {
    /// JSON merge patch — các field ở top-level sẽ override.
    pub patch: serde_json::Value,
}

#[tauri::command]
pub async fn config_set(
    state: State<'_, Arc<AppState>>,
    input: ConfigPatchInput,
) -> IpcResult<AppConfig> {
    let store = ConfigStore::new(crate::config::paths::config_path());
    let new_cfg = store
        .patch(|cfg| {
            // Apply patch — simple top-level merge for known sections
            if let Some(obj) = input.patch.as_object() {
                if let Some(agent) = obj.get("agent").and_then(|v| v.as_object()) {
                    if let Some(max_iter) = agent.get("max_iterations").and_then(|v| v.as_u64()) {
                        cfg.agent.max_iterations = max_iter as u32;
                    }
                    if let Some(max_tc) = agent.get("max_tool_calls").and_then(|v| v.as_u64()) {
                        cfg.agent.max_tool_calls = max_tc as u32;
                    }
                    if let Some(p) = agent.get("default_provider").and_then(|v| v.as_str()) {
                        cfg.agent.default_provider = p.to_string();
                    }
                    if let Some(m) = agent.get("default_model").and_then(|v| v.as_str()) {
                        cfg.agent.default_model = m.to_string();
                    }
                    if let Some(sp) = agent.get("system_prompt").and_then(|v| v.as_str()) {
                        cfg.agent.system_prompt = Some(sp.to_string());
                    }
                }
                if let Some(sec) = obj.get("security").and_then(|v| v.as_object()) {
                    if let Some(t) = sec.get("approval_timeout_secs").and_then(|v| v.as_u64()) {
                        cfg.security.approval_timeout_secs = t;
                    }
                    if let Some(t) = sec.get("shell_timeout_secs").and_then(|v| v.as_u64()) {
                        cfg.security.shell_timeout_secs = t;
                    }
                    if let Some(t) = sec.get("shell_max_output_kb").and_then(|v| v.as_u64()) {
                        cfg.security.shell_max_output_kb = t as usize;
                    }
                }
            }
        })
        .await
        .map_err(IpcError::from)?;
    Ok(new_cfg)
}
