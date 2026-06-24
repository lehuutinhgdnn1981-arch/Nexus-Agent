//! Scheduler job types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique job ID.
pub type JobId = String;

/// Kind of scheduled job.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    /// Fire 1 lần tại `fire_at`.
    OneTime { fire_at: DateTime<Utc> },
    /// Lặp lại theo cron expression (5-field: min hour dom mon dow).
    Recurring { cron: String },
}

/// Job specification — nội dung sẽ được persist vào DB + đăng ký với scheduler runtime.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobSpec {
    pub id: JobId,
    pub kind: JobKind,
    /// Message sẽ được inject vào agent khi job fire.
    pub message: String,
    /// Session để inject vào (None = default scheduled session).
    pub session_id: Option<String>,
    /// Created timestamp.
    pub created_at: DateTime<Utc>,
    /// Enabled?
    pub enabled: bool,
}

impl JobSpec {
    #[must_use]
    pub fn one_time(id: impl Into<String>, fire_at: DateTime<Utc>, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: JobKind::OneTime { fire_at },
            message: message.into(),
            session_id: None,
            created_at: Utc::now(),
            enabled: true,
        }
    }

    #[must_use]
    pub fn recurring(id: impl Into<String>, cron: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: JobKind::Recurring { cron: cron.into() },
            message: message.into(),
            session_id: None,
            created_at: Utc::now(),
            enabled: true,
        }
    }
}
