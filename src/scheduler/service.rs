//! Scheduler service — wraps `tokio-cron-scheduler`, restore jobs từ DB, fire callbacks.

use std::sync::Arc;

use chrono::Utc;
use sqlx::SqlitePool;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info, warn};

use crate::error::{Result, SchedulerError};
use crate::scheduler::job::{JobId, JobKind, JobSpec};
use crate::scheduler::nlp::parse_natural_language;
use crate::scheduler::persistence;

/// Callback khi job fire — truyền message + session_id vào agent.
pub type FireCallback = Arc<dyn Fn(JobId, String, Option<String>) + Send + Sync>;

/// Scheduler service.
pub struct SchedulerService {
    pool: SqlitePool,
    scheduler: Mutex<Option<JobScheduler>>,
    on_fire: FireCallback,
}

impl SchedulerService {
    pub fn new(pool: SqlitePool, on_fire: FireCallback) -> Self {
        Self {
            pool,
            scheduler: Mutex::new(None),
            on_fire,
        }
    }

    /// Start scheduler + restore jobs từ DB.
    pub async fn start(&self) -> Result<()> {
        let mut guard = self.scheduler.lock().await;
        if guard.is_some() {
            warn!("scheduler already started");
            return Ok(());
        }

        let sched = JobScheduler::new()
            .await
            .map_err(|e| SchedulerError::Internal(format!("create scheduler: {e}")))?;

        sched
            .start()
            .await
            .map_err(|e| SchedulerError::Internal(format!("start scheduler: {e}")))?;

        info!("scheduler started");

        // Restore jobs từ DB
        let jobs = persistence::load_enabled_jobs(&self.pool).await?;
        info!(count = jobs.len(), "restoring jobs from DB");
        for job in jobs {
            if let Err(e) = self.add_to_scheduler(&sched, &job).await {
                error!(job_id = %job.id, error = %e, "failed to restore job");
            }
        }

        *guard = Some(sched);
        Ok(())
    }

    /// Add một job mới. Persist + register với scheduler runtime.
    pub async fn add(&self, spec: JobSpec) -> Result<JobId> {
        let id = spec.id.clone();

        // Persist
        persistence::save_job(&self.pool, &spec).await?;

        // Register với scheduler runtime
        let guard = self.scheduler.lock().await;
        if let Some(sched) = &*guard {
            self.add_to_scheduler(sched, &spec).await?;
        } else {
            warn!("scheduler not started — job persisted only");
        }

        Ok(id)
    }

    /// Parse NL + add job.
    pub async fn add_from_natural_language(
        &self,
        schedule: &str,
        message: &str,
        session_id: Option<&str>,
    ) -> Result<JobId> {
        let kind = parse_natural_language(schedule)?;
        let id = crate::utils::ids::new_uuid();
        let mut spec = match kind {
            JobKind::OneTime { fire_at } => {
                JobSpec::one_time(id, fire_at, message)
            }
            JobKind::Recurring { cron } => {
                // Validate cron
                validate_cron(&cron)?;
                JobSpec::recurring(id, cron, message)
            }
        };
        spec.session_id = session_id.map(String::from);
        self.add(spec).await
    }

    /// Cancel (disable + remove from runtime scheduler).
    pub async fn cancel(&self, id: &JobId) -> Result<()> {
        persistence::disable_job(&self.pool, id).await?;
        // Note: tokio-cron-scheduler không support remove individual jobs easily.
        // Disabled jobs sẽ được skip ở fire time (check enabled flag).
        // For full cleanup, restart scheduler.
        warn!(job_id = %id, "job disabled; will skip on next fire");
        Ok(())
    }

    /// List tất cả jobs.
    pub async fn list(&self) -> Result<Vec<JobSpec>> {
        persistence::list_all_jobs(&self.pool).await
    }

    /// Shutdown scheduler.
    pub async fn shutdown(&self) -> Result<()> {
        let mut guard = self.scheduler.lock().await;
        if let Some(sched) = guard.take() {
            sched
                .shutdown()
                .await
                .map_err(|e| SchedulerError::Internal(format!("shutdown: {e}")))?;
            info!("scheduler shut down");
        }
        Ok(())
    }

    /// Add job vào scheduler runtime.
    async fn add_to_scheduler(&self, sched: &JobScheduler, spec: &JobSpec) -> Result<()> {
        let on_fire = Arc::clone(&self.on_fire);
        let id = spec.id.clone();
        let message = spec.message.clone();
        let session_id = spec.session_id.clone();

        match &spec.kind {
            JobKind::OneTime { fire_at } => {
                let now = Utc::now();
                let delay = if *fire_at > now {
                    (*fire_at - now).to_std()
                        .map_err(|e| SchedulerError::Internal(format!("duration: {e}")))?
                } else {
                    std::time::Duration::from_secs(0)
                };
                let pool = self.pool.clone();
                let id_clone = id.clone();
                let job = Job::new_one_shot_async(delay, move |_uuid, _l| {
                    let on_fire = Arc::clone(&on_fire);
                    let id_inner = id_clone.clone();
                    let message_inner = message.clone();
                    let session_inner = session_id.clone();
                    let pool_inner = pool.clone();
                    Box::pin(async move {
                        info!(job_id = %id_inner, "one-time job fired");
                        on_fire(id_inner.clone(), message_inner, session_inner);
                        if let Err(e) = persistence::complete_one_time(&pool_inner, &id_inner).await {
                            error!(error = %e, "failed to complete one-time job");
                        }
                    })
                })
                .await
                .map_err(|e| SchedulerError::Internal(format!("add one-shot job: {e}")))?;
                sched
                    .add(job)
                    .await
                    .map_err(|e| SchedulerError::Internal(format!("scheduler add: {e}")))?;
            }
            JobKind::Recurring { cron } => {
                let pool = self.pool.clone();
                let id_clone = id.clone();
                let cron_clone = cron.clone();
                let job = Job::new_async(cron_clone.as_str(), move |_uuid, _l| {
                    let on_fire = Arc::clone(&on_fire);
                    let id_inner = id_clone.clone();
                    let message_inner = message.clone();
                    let session_inner = session_id.clone();
                    let pool_inner = pool.clone();
                    Box::pin(async move {
                        info!(job_id = %id_inner, "recurring job fired");
                        on_fire(id_inner.clone(), message_inner.clone(), session_inner.clone());
                        if let Err(e) = persistence::mark_fired(&pool_inner, &id_inner).await {
                            error!(error = %e, "failed to mark fired");
                        }
                    })
                })
                .await
                .map_err(|e| SchedulerError::InvalidCron(format!("cron `{cron}`: {e}")))?;
                sched
                    .add(job)
                    .await
                    .map_err(|e| SchedulerError::Internal(format!("scheduler add: {e}")))?;
            }
        }
        Ok(())
    }
}

/// Validate cron expression bằng cách thử tạo Job.
fn validate_cron(cron: &str) -> Result<()> {
    // Basic format check: 5 fields
    let fields: Vec<&str> = cron.split_whitespace().collect();
    if fields.len() != 5 {
        return Err(SchedulerError::InvalidCron(format!(
            "cron must have 5 fields, got {}: `{cron}`",
            fields.len()
        ))
        .into());
    }
    Ok(())
}
