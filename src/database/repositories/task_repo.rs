//! Task repository (scheduled jobs).

use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::{error::Result, utils::time::now_ts};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRow {
    pub id: String,
    pub kind: String,                       // 'one_time' | 'recurring'
    pub payload: String,                    // JSON JobSpec
    pub cron: Option<String>,
    pub fire_at: Option<i64>,
    pub enabled: bool,
    pub created_at: i64,
    pub last_fired_at: Option<i64>,
}

fn from_row(row: SqliteRow) -> TaskRow {
    let enabled_int: i64 = row.get("enabled");
    TaskRow {
        id: row.get("id"),
        kind: row.get("kind"),
        payload: row.get("payload"),
        cron: row.get("cron"),
        fire_at: row.get("fire_at"),
        enabled: enabled_int != 0,
        created_at: row.get("created_at"),
        last_fired_at: row.get("last_fired_at"),
    }
}

pub struct TaskRepo;

impl TaskRepo {
    pub async fn insert(pool: &SqlitePool, t: &TaskRow) -> Result<()> {
        let now = now_ts();
        let enabled_int: i64 = if t.enabled { 1 } else { 0 };
        sqlx::query(
            "INSERT INTO tasks (id, kind, payload, cron, fire_at, enabled, created_at, last_fired_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&t.id)
        .bind(&t.kind)
        .bind(&t.payload)
        .bind(&t.cron)
        .bind(t.fire_at)
        .bind(enabled_int)
        .bind(now)
        .bind(t.last_fired_at)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get(pool: &SqlitePool, id: &str) -> Result<Option<TaskRow>> {
        let row = sqlx::query("SELECT * FROM tasks WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        Ok(row.map(from_row))
    }

    pub async fn list_enabled(pool: &SqlitePool) -> Result<Vec<TaskRow>> {
        let rows = sqlx::query("SELECT * FROM tasks WHERE enabled = 1 ORDER BY created_at ASC")
            .fetch_all(pool)
            .await?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn list_all(pool: &SqlitePool) -> Result<Vec<TaskRow>> {
        let rows = sqlx::query("SELECT * FROM tasks ORDER BY created_at ASC")
            .fetch_all(pool)
            .await?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn mark_fired(pool: &SqlitePool, id: &str) -> Result<()> {
        let now = now_ts();
        sqlx::query("UPDATE tasks SET last_fired_at = ? WHERE id = ?")
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn disable(pool: &SqlitePool, id: &str) -> Result<()> {
        sqlx::query("UPDATE tasks SET enabled = 0 WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn task_roundtrip() {
        let pool = crate::database::pool::in_memory_pool().await.unwrap();
        let t = TaskRow {
            id: "t1".into(),
            kind: "one_time".into(),
            payload: "{}".into(),
            cron: None,
            fire_at: Some(now_ts() + 3600),
            enabled: true,
            created_at: now_ts(),
            last_fired_at: None,
        };
        TaskRepo::insert(&pool, &t).await.unwrap();
        let list = TaskRepo::list_enabled(&pool).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "t1");

        TaskRepo::disable(&pool, "t1").await.unwrap();
        let list2 = TaskRepo::list_enabled(&pool).await.unwrap();
        assert!(list2.is_empty());
    }
}
