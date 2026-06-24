//! Browser IPC commands (manual mode).

use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

use crate::browser::page::{execute, PageAction};
use crate::state::AppState;

use super::{IpcError, IpcResult};

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BrowserActionInput {
    Navigate { url: String },
    Click { selector: String },
    Type { selector: String, text: String },
    Wait { selector: String },
    ExtractText,
    Screenshot { full_page: Option<bool> },
}

#[tauri::command]
pub async fn browser_action(
    state: State<'_, Arc<AppState>>,
    action: BrowserActionInput,
) -> IpcResult<serde_json::Value> {
    let page = state.browser.page().await.map_err(IpcError::from)?;
    let page_action = match action {
        BrowserActionInput::Navigate { url } => PageAction::Navigate { url, timeout_ms: 30_000 },
        BrowserActionInput::Click { selector } => PageAction::Click { selector, timeout_ms: 30_000 },
        BrowserActionInput::Type { selector, text } => {
            PageAction::Type { selector, text, delay_ms: 0 }
        }
        BrowserActionInput::Wait { selector } => PageAction::Wait { selector, timeout_ms: 30_000 },
        BrowserActionInput::ExtractText => PageAction::ExtractText,
        BrowserActionInput::Screenshot { full_page } => PageAction::Screenshot {
            full_page: full_page.unwrap_or(false),
        },
    };
    let result = execute(&page, &page_action).await.map_err(IpcError::from)?;
    Ok(result)
}

#[tauri::command]
pub async fn browser_shutdown(state: State<'_, Arc<AppState>>) -> IpcResult<()> {
    state.browser.shutdown().await.map_err(IpcError::from)?;
    Ok(())
}
