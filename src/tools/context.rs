//! Tool context — passed vào `Tool::execute`.

use std::sync::Arc;

use sqlx::SqlitePool;

use crate::browser::BrowserManager;
use crate::config::AppConfig;
use crate::memory::store::MemoryStore;
use crate::scheduler::SchedulerService;
use crate::security::sandbox::Sandbox;

/// Context cung cấp các shared dependency cho tool.
#[derive(Clone)]
pub struct ToolContext {
    pub session_id: Option<String>,
    pub run_id: Option<String>,
    pub workspace: Arc<Sandbox>,
    pub pool: SqlitePool,
    pub memory: Arc<MemoryStore>,
    pub browser: Arc<BrowserManager>,
    pub scheduler: Arc<SchedulerService>,
    pub config: Arc<AppConfig>,
}
