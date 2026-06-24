//! Chat IPC commands.

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;
use tracing::warn;

use crate::agent::agent::Agent;
use crate::agent::brain::Brain;
use crate::agent::config::AgentRuntimeConfig;
use crate::agent::event::AgentEvent;
use crate::llm::factory::build_provider_from_app_config;
use crate::state::AppState;

use super::{IpcError, IpcResult};

#[derive(Debug, Deserialize)]
pub struct ChatSendInput {
    pub session_id: String,
    pub message: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    /// Bật V2 brain (memory tiering + compression + reflection + episode memory).
    /// Default: true.
    #[serde(default = "default_true")]
    pub enable_brain: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct ChatSendOutput {
    pub run_id: String,
}

#[tauri::command]
pub async fn chat_send(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: ChatSendInput,
) -> IpcResult<ChatSendOutput> {
    let provider_name = input
        .provider
        .clone()
        .unwrap_or_else(|| state.config.agent.default_provider.clone());
    let model = input
        .model
        .clone()
        .unwrap_or_else(|| state.config.agent.default_model.clone());

    // Build provider via factory (hỗ trợ cả 4 built-in + custom providers)
    let provider = build_provider_from_app_config(&provider_name, &state.config, None)
        .map_err(IpcError::from)?;

    let mut rt_config = AgentRuntimeConfig::default();
    rt_config.default_provider = provider_name.clone();
    rt_config.default_model = model.clone();
    rt_config.max_iterations = state.config.agent.max_iterations;
    rt_config.max_tool_calls = state.config.agent.max_tool_calls;
    rt_config.system_prompt = state.config.agent.system_prompt.clone();

    let mut agent = Agent::new(Arc::clone(&state), rt_config, provider);

    // === V2: Build Brain module (optional) ===
    if input.enable_brain {
        match Brain::new(
            // Re-build a separate provider Arc for brain (avoid moving the one used by agent)
            build_provider_from_app_config(&provider_name, &state.config, None)
                .map_err(IpcError::from)?,
            state.pool.clone(),
            &state.tool_registry,
            model.clone(),
            crate::agent::brain::ContextConfig::default(),
        )
        .await
        {
            Ok(brain) => {
                agent = agent.with_brain(Arc::new(brain));
            }
            Err(e) => {
                warn!(error = %e, "failed to build Brain — continuing without V2 features");
            }
        }
    }

    let (tx, mut rx) = mpsc::channel::<AgentEvent>(128);

    // Spawn forwarder: AgentEvent → Tauri IPC event
    let app_clone = Arc::new(app.clone());
    let session_id_clone = input.session_id.clone();
    tokio::spawn(async move {
        while let Some(ev) = rx.recv().await {
            let event_name = match &ev {
                AgentEvent::TurnStart { .. } => "agent:turn_start",
                AgentEvent::Delta { .. } => "agent:delta",
                AgentEvent::ToolCallStart { .. } => "agent:tool_call",
                AgentEvent::ToolCallEnd { .. } => "agent:tool_result",
                AgentEvent::IterationEnd { .. } => "agent:iteration_end",
                AgentEvent::Done { .. } => "agent:done",
                AgentEvent::Error { .. } => "agent:error",
                AgentEvent::Cancelled { .. } => "agent:cancelled",
                AgentEvent::ApprovalRequest { .. } => "approval:request",
            };
            let _ = app_clone.emit(event_name, &ev);
        }
        drop(session_id_clone);
    });

    // Spawn agent turn
    let _run_id_handle = tokio::spawn(async move {
        if let Err(e) = agent.run(&input.session_id, &input.message, tx).await {
            warn!(error = %e, "agent run errored");
        }
    });

    // Note: run_id is generated inside agent.run() — we don't have it here yet.
    // Frontend nên listen event `agent:turn_start` để lấy run_id.
    Ok(ChatSendOutput {
        run_id: "(pending)".into(),
    })
}

#[tauri::command]
pub async fn chat_cancel(state: State<'_, Arc<AppState>>, run_id: String) -> IpcResult<()> {
    if let Some(entry) = state.active_runs.get(&run_id) {
        entry.cancel();
    }
    Ok(())
}
