use std::sync::Arc;
use tauri::State;
use crate::config::provider_config::ProviderConfig;
use crate::config::store::ConfigStore;
use crate::config::AppConfig;
use crate::state::AppState;
use super::{IpcError, IpcResult};

#[tauri::command]
pub async fn config_get(state: State<'_, Arc<AppState>>) -> IpcResult<AppConfig> { Ok((*state.config).clone()) }

#[derive(Debug, serde::Deserialize)]
pub struct ConfigPatchInput { pub patch: serde_json::Value }

#[tauri::command]
pub async fn config_set(_state: State<'_, Arc<AppState>>, input: ConfigPatchInput) -> IpcResult<AppConfig> {
    let store = ConfigStore::new(crate::config::paths::config_path());
    let new_cfg = store.patch(|cfg| { apply_patch(cfg, &input.patch); }).map_err(IpcError::from)?;
    Ok(new_cfg)
}

fn apply_patch(cfg: &mut AppConfig, patch: &serde_json::Value) {
    let Some(obj) = patch.as_object() else { return };
    if let Some(a) = obj.get("agent").and_then(|v| v.as_object()) {
        if let Some(v) = a.get("max_iterations").and_then(|v| v.as_u64()) { cfg.agent.max_iterations = v as u32; }
        if let Some(v) = a.get("max_tool_calls").and_then(|v| v.as_u64()) { cfg.agent.max_tool_calls = v as u32; }
        if let Some(v) = a.get("default_provider").and_then(|v| v.as_str()) { cfg.agent.default_provider = v.to_string(); }
        if let Some(v) = a.get("default_model").and_then(|v| v.as_str()) { cfg.agent.default_model = v.to_string(); }
        if let Some(v) = a.get("system_prompt").and_then(|v| v.as_str()) { cfg.agent.system_prompt = Some(v.to_string()); }
    }
    if let Some(s) = obj.get("security").and_then(|v| v.as_object()) {
        if let Some(v) = s.get("approval_timeout_secs").and_then(|v| v.as_u64()) { cfg.security.approval_timeout_secs = v; }
        if let Some(v) = s.get("shell_timeout_secs").and_then(|v| v.as_u64()) { cfg.security.shell_timeout_secs = v; }
        if let Some(v) = s.get("shell_max_output_kb").and_then(|v| v.as_u64()) { cfg.security.shell_max_output_kb = v as usize; }
    }
    if let Some(llm) = obj.get("llm").and_then(|v| v.as_object()) {
        for name in &["openai", "openrouter", "anthropic", "ollama"] {
            if let Some(po) = llm.get(*name).and_then(|v| v.as_object()) {
                let target = match *name { "openai" => &mut cfg.llm.openai, "openrouter" => &mut cfg.llm.openrouter, "anthropic" => &mut cfg.llm.anthropic, "ollama" => &mut cfg.llm.ollama, _ => continue };
                if let Some(v) = po.get("api_key") { target.api_key = v.as_str().map(|s| s.to_string()).filter(|s| !s.is_empty()); }
                if let Some(v) = po.get("base_url") { target.base_url = v.as_str().map(|s| s.to_string()).filter(|s| !s.is_empty()); }
                if let Some(v) = po.get("default_model") { target.default_model = v.as_str().map(|s| s.to_string()).filter(|s| !s.is_empty()); }
            }
        }
    }
}
