use std::path::Path;
use sqlx::{sqlite::{SqliteConnectOptions, SqlitePoolOptions}, SqlitePool};
use tracing::info;
use crate::error::Result;

pub type DbPool = SqlitePool;

pub async fn init_pool<P: AsRef<Path>>(db_path: P) -> Result<DbPool> {
    let db_path = db_path.as_ref();
    if let Some(parent) = db_path.parent() { std::fs::create_dir_all(parent).ok(); }
    let options = SqliteConnectOptions::new()
        .filename(db_path).create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true).busy_timeout(std::time::Duration::from_secs(5))
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);
    let pool = SqlitePoolOptions::new().max_connections(8).connect_with(options).await?;
    info!(path = %db_path.display(), "SQLite pool initialized");
    run_migrations(&pool).await?;
    Ok(pool)
}

pub async fn in_memory_pool() -> Result<DbPool> {
    let options = SqliteConnectOptions::new().in_memory(true).create_if_missing(true).foreign_keys(true);
    let pool = SqlitePoolOptions::new().max_connections(4).connect_with(options).await?;
    run_migrations(&pool).await?;
    Ok(pool)
}

async fn run_migrations(pool: &DbPool) -> Result<()> {
    sqlx::migrate!("./database/migrations").run(pool).await
        .map_err(|e| crate::error::NexusError::Internal(format!("migration error: {e}")))?;
    info!("database migrations applied");
    Ok(())
}
