//! Scheduler persistence — save/load jobs from `tasks` table.

use chrono::Utc;
use sqlx::SqlitePool;

use crate::database::repositories::task_repo::{TaskRepo, TaskRow};
use crate::error::{Result, SchedulerError};
use crate::scheduler::job::{JobId, JobKind, JobSpec};

/// Persist job vào DB.
pub async fn save_job(pool: &SqlitePool, job: &JobSpec) -> Result<()> {
    let payload = serde_json::to_string(job)
        .map_err(|e| SchedulerError::Internal(format!("serialize job: {e}")))?;
    let (kind_str, cron, fire_at) = match &job.kind {
        JobKind::OneTime { fire_at } => ("one_time", None, Some(fire_at.timestamp())),
        JobKind::Recurring { cron } => ("recurring", Some(cron.clone()), None),
    };

    let row = TaskRow {
        id: job.id.clone(),
        kind: kind_str.to_string(),
        payload,
        cron,
        fire_at,
        enabled: job.enabled,
        created_at: job.created_at.timestamp(),
        last_fired_at: None,
    };
    TaskRepo::insert(pool, &row).await?;
    Ok(())
}

/// Load tất cả enabled jobs từ DB.
pub async fn load_enabled_jobs(pool: &SqlitePool) -> Result<Vec<JobSpec>> {
    let rows = TaskRepo::list_enabled(pool).await?;
    let mut jobs = Vec::with_capacity(rows.len());
    for row in rows {
        let job: JobSpec = serde_json::from_str(&row.payload)
            .map_err(|e| SchedulerError::Internal(format!("deserialize job {}: {e}", row.id)))?;
        jobs.push(job);
    }
    Ok(jobs)
}

/// Mark job đã fire (update last_fired_at).
pub async fn mark_fired(pool: &SqlitePool, id: &JobId) -> Result<()> {
    TaskRepo::mark_fired(pool, id).await?;
    Ok(())
}

/// Disable job (giữ row nhưng enabled = 0).
pub async fn disable_job(pool: &SqlitePool, id: &JobId) -> Result<()> {
    TaskRepo::disable(pool, id).await?;
    Ok(())
}

/// Xóa job khỏi DB.
pub async fn delete_job(pool: &SqlitePool, id: &JobId) -> Result<()> {
    TaskRepo::delete(pool, id).await?;
    Ok(())
}

/// Lấy job theo ID.
pub async fn get_job(pool: &SqlitePool, id: &JobId) -> Result<Option<JobSpec>> {
    let row = TaskRepo::get(pool, id).await?;
    if let Some(r) = row {
        let job: JobSpec = serde_json::from_str(&r.payload)
            .map_err(|e| SchedulerError::Internal(format!("deserialize job {id}: {e}")))?;
        Ok(Some(job))
    } else {
        Ok(None)
    }
}

/// List tất cả jobs.
pub async fn list_all_jobs(pool: &SqlitePool) -> Result<Vec<JobSpec>> {
    let rows = TaskRepo::list_all(pool).await?;
    let mut jobs = Vec::with_capacity(rows.len());
    for row in rows {
        let job: JobSpec = serde_json::from_str(&row.payload)
            .map_err(|e| SchedulerError::Internal(format!("deserialize job {}: {e}", row.id)))?;
        jobs.push(job);
    }
    Ok(jobs)
}

/// Tạo OneTime job đã hoàn thành (để không fire lại).
pub async fn complete_one_time(pool: &SqlitePool, id: &JobId) -> Result<()> {
    let _ = Utc::now();
    disable_job(pool, id).await?;
    mark_fired(pool, id).await?;
    Ok(())
}
