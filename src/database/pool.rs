//! SQLite connection pool + migration runner.

use std::path::Path;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};
use tracing::info;

use crate::error::Result;

/// Alias cho SqlitePool.
pub type DbPool = SqlitePool;

/// Khởi tạo connection pool đến SQLite file tại `db_path`.
///
/// - Tạo file + thư mục cha nếu chưa tồn tại.
/// - Bật WAL mode + foreign_keys + busy_timeout 5s.
/// - Chạy toàn bộ migrations nhúng ở `migrations/`.
pub async fn init_pool<P: AsRef<Path>>(db_path: P) -> Result<DbPool> {
    let db_path = db_path.as_ref();
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

    let options = SqliteConnectOptions::from_url(&db_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(std::time::Duration::from_secs(5))
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await?;

    info!(path = %db_path.display(), "SQLite pool initialized");

    run_migrations(&pool).await?;

    Ok(pool)
}

/// Tạo in-memory pool (dùng cho test).
pub async fn in_memory_pool() -> Result<DbPool> {
    let options = SqliteConnectOptions::from_url("sqlite::memory:")
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;

    run_migrations(&pool).await?;
    Ok(pool)
}

/// Chạy embedded migrations.
async fn run_migrations(pool: &DbPool) -> Result<()> {
    sqlx::migrate!("./database/migrations")
        .run(pool)
        .await
        .map_err(|e| crate::error::NexusError::Internal(format!("migration error: {e}")))?;
    info!("database migrations applied");
    Ok(())
}
