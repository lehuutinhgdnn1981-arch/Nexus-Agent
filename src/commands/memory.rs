//! Memory IPC commands.

use serde::{Deserialize, Serialize};
use tauri::State;
use std::sync::Arc;

use crate::memory::model::{MemoryCategory, MemoryEntry, MemoryQuery};
use crate::state::AppState;

use super::{IpcError, IpcResult};

#[derive(Debug, Deserialize)]
pub struct MemorySaveInput {
    pub content: String,
    pub category: String,
    pub tags: Option<Vec<String>>,
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MemoryDto {
    pub id: String,
    pub content: String,
    pub category: String,
    pub tags: Vec<String>,
    pub session_id: Option<String>,
    pub created_at: i64,
    pub last_used_at: i64,
    pub use_count: u32,
}

impl From<MemoryEntry> for MemoryDto {
    fn from(m: MemoryEntry) -> Self {
        Self {
            id: m.id,
            content: m.content,
            category: m.category.as_str().to_string(),
            tags: m.tags,
            session_id: m.session_id,
            created_at: m.created_at,
            last_used_at: m.last_used_at,
            use_count: m.use_count,
        }
    }
}

#[tauri::command]
pub async fn memory_save(
    state: State<'_, Arc<AppState>>,
    input: MemorySaveInput,
) -> IpcResult<String> {
    let category = MemoryCategory::from_str(&input.category);
    let id = state
        .memory
        .save_long_term(&input.content, category, input.tags.unwrap_or_default(), input.session_id.as_deref())
        .await
        .map_err(IpcError::from)?;
    Ok(id)
}

#[derive(Debug, Deserialize)]
pub struct MemoryRecallInput {
    pub query: String,
    pub top_k: Option<u32>,
    pub category: Option<String>,
}

#[tauri::command]
pub async fn memory_recall(
    state: State<'_, Arc<AppState>>,
    input: MemoryRecallInput,
) -> IpcResult<Vec<MemoryDto>> {
    let mut q = MemoryQuery::new(&input.query);
    q.top_k = input.top_k.unwrap_or(5);
    q.category = input.category.as_deref().map(MemoryCategory::from_str);
    let results = state.memory.recall(&q).await.map_err(IpcError::from)?;
    Ok(results.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn memory_list(
    state: State<'_, Arc<AppState>>,
    limit: Option<i64>,
) -> IpcResult<Vec<MemoryDto>> {
    let rows = state
        .memory
        .list_recent(limit.unwrap_or(100))
        .await
        .map_err(IpcError::from)?;
    Ok(rows.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn memory_delete(state: State<'_, Arc<AppState>>, id: String) -> IpcResult<()> {
    state.memory.delete(&id).await.map_err(IpcError::from)?;
    Ok(())
}
