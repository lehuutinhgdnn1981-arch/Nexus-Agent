//! Custom provider IPC commands — runtime add/remove/list custom LLM providers.

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::llm::custom::CustomProviderConfig;
use crate::state::AppState;

use super::{IpcError, IpcResult};

#[derive(Debug, Serialize)]
pub struct CustomProviderDto {
    pub id: String,
    pub base_url: String,
    pub default_model: Option<String>,
    pub display_name: Option<String>,
    pub supports_tools: bool,
    pub has_api_key: bool,
}

impl From<&CustomProviderConfig> for CustomProviderDto {
    fn from(c: &CustomProviderConfig) -> Self {
        Self {
            id: c.id.clone(),
            base_url: c.base_url.clone(),
            default_model: c.default_model.clone(),
            display_name: c.display_name.clone(),
            supports_tools: c.supports_tools,
            has_api_key: c.resolved_api_key().is_some(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AddCustomProviderInput {
    pub config: CustomProviderConfig,
}

#[tauri::command]
pub async fn custom_provider_add(
    state: State<'_, Arc<AppState>>,
    input: AddCustomProviderInput,
) -> IpcResult<()> {
    let id = input.config.id.clone();
    if id.is_empty() {
        return Err(IpcError {
            code: "invalid_argument".into(),
            message: "provider id cannot be empty".into(),
        });
    }

    // Validate: ensure provider can be constructed
    if let Err(e) = crate::llm::custom::CustomProvider::new(input.config.clone()) {
        return Err(IpcError {
            code: "invalid_argument".into(),
            message: format!("invalid provider config: {e}"),
        });
    }

    // Persist vào config file
    let store = crate::config::ConfigStore::new(crate::config::paths::config_path());
    store
        .patch(|cfg| {
            cfg.llm.custom.insert(id.clone(), input.config.clone());
        })
        .map_err(IpcError::from)?;

    // Update in-memory state
    let mut new_config = (*state.config).clone();
    new_config.llm.custom.insert(id, input.config);
    // Note: state.config là Arc<AppConfig> immutable — for runtime add,
    // users currently need to restart app to pick up new provider in agent.
    // Future: switch to Arc<RwLock<AppConfig>> for true hot-reload.
    let _ = new_config;

    Ok(())
}

#[tauri::command]
pub async fn custom_provider_remove(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> IpcResult<()> {
    let store = crate::config::ConfigStore::new(crate::config::paths::config_path());
    store
        .patch(|cfg| {
            cfg.llm.custom.remove(&id);
        })
        .map_err(IpcError::from)?;

    let _ = state;
    Ok(())
}

#[tauri::command]
pub async fn custom_provider_list(
    state: State<'_, Arc<AppState>>,
) -> IpcResult<Vec<CustomProviderDto>> {
    let providers: Vec<CustomProviderDto> = state
        .config
        .llm
        .custom
        .values()
        .map(CustomProviderDto::from)
        .collect();
    Ok(providers)
}

#[tauri::command]
pub async fn provider_list_all(
    state: State<'_, Arc<AppState>>,
) -> IpcResult<Vec<String>> {
    Ok(state.config.all_provider_ids())
}

/// Helper constructors — generate config cho common providers (gọi từ UI).
#[tauri::command]
pub async fn custom_provider_preset(
    _state: State<'_, Arc<AppState>>,
    preset_id: String,
    api_key: Option<String>,
) -> IpcResult<CustomProviderConfig> {
    let cfg = match preset_id.as_str() {
        "together" => {
            let key = api_key
                .or_else(|| std::env::var("TOGETHER_API_KEY").ok())
                .ok_or_else(|| IpcError {
                    code: "invalid_argument".into(),
                    message: "TOGETHER_API_KEY not provided".into(),
                })?;
            CustomProviderConfig::together(key)
        }
        "groq" => {
            let key = api_key
                .or_else(|| std::env::var("GROQ_API_KEY").ok())
                .ok_or_else(|| IpcError {
                    code: "invalid_argument".into(),
                    message: "GROQ_API_KEY not provided".into(),
                })?;
            CustomProviderConfig::groq(key)
        }
        "mistral" => {
            let key = api_key
                .or_else(|| std::env::var("MISTRAL_API_KEY").ok())
                .ok_or_else(|| IpcError {
                    code: "invalid_argument".into(),
                    message: "MISTRAL_API_KEY not provided".into(),
                })?;
            CustomProviderConfig::mistral(key)
        }
        "deepseek" => {
            let key = api_key
                .or_else(|| std::env::var("DEEPSEEK_API_KEY").ok())
                .ok_or_else(|| IpcError {
                    code: "invalid_argument".into(),
                    message: "DEEPSEEK_API_KEY not provided".into(),
                })?;
            CustomProviderConfig::deepseek(key)
        }
        "vllm" => CustomProviderConfig::vllm(
            "http://localhost:8000/v1".into(),
            "meta-llama/Llama-3-8B".into(),
        ),
        "lmstudio" => CustomProviderConfig::lm_studio(),
        "litellm" => CustomProviderConfig::litellm(
            "http://localhost:4000/v1".into(),
            std::env::var("LITELLM_API_KEY").ok(),
        ),
        other => {
            return Err(IpcError {
                code: "invalid_argument".into(),
                message: format!(
                    "unknown preset: `{other}`. Available: together, groq, mistral, deepseek, vllm, lmstudio, litellm"
                ),
            });
        }
    };
    Ok(cfg)
}
