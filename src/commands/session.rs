//! Session IPC commands.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::database::repositories::session_repo::{SessionRepo, SessionRow};

use super::{AppStateRef, IpcError, IpcResult};

#[derive(Debug, Deserialize)]
pub struct CreateSessionInput {
    pub title: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionDto {
    pub id: String,
    pub title: String,
    pub provider: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<SessionRow> for SessionDto {
    fn from(r: SessionRow) -> Self {
        Self {
            id: r.id,
            title: r.title,
            provider: r.provider,
            model: r.model,
            system_prompt: r.system_prompt,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[tauri::command]
pub async fn session_create(
    state: AppStateRef<'_>,
    input: CreateSessionInput,
) -> IpcResult<SessionDto> {
    let id = crate::utils::ids::new_uuid();
    let title = input.title.unwrap_or_else(|| "New Session".into());
    let provider = input.provider.unwrap_or_else(|| state.config.agent.default_provider.clone());
    let model = input.model.unwrap_or_else(|| state.config.agent.default_model.clone());

    let row = SessionRepo::create(
        &state.pool,
        &id,
        &title,
        &provider,
        &model,
        input.system_prompt.as_deref(),
    )
    .await
    .map_err(IpcError::from)?;

    Ok(row.into())
}

#[tauri::command]
pub async fn session_list(state: AppStateRef<'_>, limit: Option<i64>) -> IpcResult<Vec<SessionDto>> {
    let limit = limit.unwrap_or(100);
    let rows = SessionRepo::list(&state.pool, limit)
        .await
        .map_err(IpcError::from)?;
    Ok(rows.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn session_search(
    state: AppStateRef<'_>,
    query: String,
    limit: Option<i64>,
) -> IpcResult<Vec<SessionDto>> {
    let rows = SessionRepo::search(&state.pool, &query, limit.unwrap_or(50))
        .await
        .map_err(IpcError::from)?;
    Ok(rows.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn session_rename(
    state: AppStateRef<'_>,
    id: String,
    title: String,
) -> IpcResult<()> {
    SessionRepo::rename(&state.pool, &id, &title)
        .await
        .map_err(IpcError::from)?;
    Ok(())
}

#[tauri::command]
pub async fn session_delete(state: AppStateRef<'_>, id: String) -> IpcResult<()> {
    SessionRepo::delete(&state.pool, &id)
        .await
        .map_err(IpcError::from)?;
    state.memory.drop_session(&id);
    Ok(())
}

/// Wire toàn bộ session commands vào Tauri builder.
pub fn invoke_handlers() -> Vec<&'static str> {
    vec![
        "session_create",
        "session_list",
        "session_search",
        "session_rename",
        "session_delete",
    ]
}
