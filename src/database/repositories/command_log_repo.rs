//! Command log repository (audit trail cho shell/exec).

use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::{error::Result, utils::time::now_ts};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandLogRow {
    pub id: String,
    pub session_id: Option<String>,
    pub command: String,
    pub args: String,                       // JSON array
    pub status: String,                     // 'approved' | 'rejected' | 'blacklisted' | 'executed' | 'timeout' | 'error'
    pub exit_code: Option<i64>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub started_at: i64,
    pub finished_at: Option<i64>,
}

fn from_row(row: SqliteRow) -> CommandLogRow {
    CommandLogRow {
        id: row.get("id"),
        session_id: row.get("session_id"),
        command: row.get("command"),
        args: row.get("args"),
        status: row.get("status"),
        exit_code: row.get("exit_code"),
        stdout: row.get("stdout"),
        stderr: row.get("stderr"),
        started_at: row.get("started_at"),
        finished_at: row.get("finished_at"),
    }
}

pub struct CommandLogRepo;

impl CommandLogRepo {
    pub async fn insert(pool: &SqlitePool, row: &CommandLogRow) -> Result<()> {
        sqlx::query(
            "INSERT INTO command_logs \
             (id, session_id, command, args, status, exit_code, stdout, stderr, started_at, finished_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&row.id)
        .bind(&row.session_id)
        .bind(&row.command)
        .bind(&row.args)
        .bind(&row.status)
        .bind(row.exit_code)
        .bind(&row.stdout)
        .bind(&row.stderr)
        .bind(row.started_at)
        .bind(row.finished_at)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn list_by_session(
        pool: &SqlitePool,
        session_id: &str,
        limit: i64,
    ) -> Result<Vec<CommandLogRow>> {
        let rows = sqlx::query(
            "SELECT * FROM command_logs WHERE session_id = ? ORDER BY started_at DESC LIMIT ?",
        )
        .bind(session_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn list_recent(pool: &SqlitePool, limit: i64) -> Result<Vec<CommandLogRow>> {
        let rows = sqlx::query("SELECT * FROM command_logs ORDER BY started_at DESC LIMIT ?")
            .bind(limit)
            .fetch_all(pool)
            .await?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn update_result(
        pool: &SqlitePool,
        id: &str,
        status: &str,
        exit_code: Option<i64>,
        stdout: Option<&str>,
        stderr: Option<&str>,
    ) -> Result<()> {
        let now = now_ts();
        sqlx::query(
            "UPDATE command_logs SET status = ?, exit_code = ?, stdout = ?, stderr = ?, finished_at = ? \
             WHERE id = ?",
        )
        .bind(status)
        .bind(exit_code)
        .bind(stdout)
        .bind(stderr)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }
}
