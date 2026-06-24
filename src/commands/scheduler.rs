//! Scheduler IPC commands.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

use crate::scheduler::job::JobSpec;
use crate::state::AppState;

use super::{IpcError, IpcResult};

#[derive(Debug, Deserialize)]
pub struct SchedulerAddInput {
    pub schedule: String,
    pub message: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SchedulerJobDto {
    pub id: String,
    pub kind: String,
    pub message: String,
    pub session_id: Option<String>,
    pub enabled: bool,
    pub created_at: i64,
}

impl From<JobSpec> for SchedulerJobDto {
    fn from(j: JobSpec) -> Self {
        let kind = match j.kind {
            crate::scheduler::job::JobKind::OneTime { fire_at } => {
                format!("one_time @ {}", fire_at.to_rfc3339())
            }
            crate::scheduler::job::JobKind::Recurring { cron } => {
                format!("recurring {cron}")
            }
        };
        Self {
            id: j.id,
            kind,
            message: j.message,
            session_id: j.session_id,
            enabled: j.enabled,
            created_at: j.created_at.timestamp(),
        }
    }
}

#[tauri::command]
pub async fn scheduler_add(
    state: State<'_, Arc<AppState>>,
    input: SchedulerAddInput,
) -> IpcResult<String> {
    let id = state
        .scheduler
        .add_from_natural_language(&input.schedule, &input.message, input.session_id.as_deref())
        .await
        .map_err(IpcError::from)?;
    Ok(id)
}

#[tauri::command]
pub async fn scheduler_list(state: State<'_, Arc<AppState>>) -> IpcResult<Vec<SchedulerJobDto>> {
    let jobs = state.scheduler.list().await.map_err(IpcError::from)?;
    Ok(jobs.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn scheduler_cancel(state: State<'_, Arc<AppState>>, id: String) -> IpcResult<()> {
    state.scheduler.cancel(&id).await.map_err(IpcError::from)?;
    Ok(())
}
