//! NEXUS — IPC commands (Tauri #[command] handlers).

pub mod approval;
pub mod browser;
pub mod chat;
pub mod config;
pub mod custom_provider;
pub mod file_upload;
pub mod memory;
pub mod palette;
pub mod scheduler;
pub mod session;
pub mod tool;

use serde::Serialize;
use tauri::State;

use crate::error::{error_to_ipc_payload, NexusError};
use crate::state::AppState;

/// IPC error type — Tauri auto-converts via Serialize.
#[derive(Debug, Serialize)]
pub struct IpcError {
    pub code: String,
    pub message: String,
}

impl From<NexusError> for IpcError {
    fn from(e: NexusError) -> Self {
        let payload = error_to_ipc_payload(&e);
        Self {
            code: payload
                .get("code")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            message: payload
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
        }
    }
}

pub type IpcResult<T> = std::result::Result<T, IpcError>;

/// Shared type alias cho Tauri State<AppState>.
pub type AppStateRef<'a> = State<'a, std::sync::Arc<AppState>>;
