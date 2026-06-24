//! Session repository.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::{error::Result, utils::time::now_ts};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRow {
    pub id: String,
    pub title: String,
    pub provider: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

fn from_row(row: SqliteRow) -> SessionRow {
    SessionRow {
        id: row.get("id"),
        title: row.get("title"),
        provider: row.get("provider"),
        model: row.get("model"),
        system_prompt: row.get("system_prompt"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub struct SessionRepo;

impl SessionRepo {
    pub async fn create(
        pool: &SqlitePool,
        id: &str,
        title: &str,
        provider: &str,
        model: &str,
        system_prompt: Option<&str>,
    ) -> Result<SessionRow> {
        let now = now_ts();
        sqlx::query(
            "INSERT INTO sessions (id, title, provider, model, system_prompt, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(title)
        .bind(provider)
        .bind(model)
        .bind(system_prompt)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        Self::get(pool, id).await
    }

    pub async fn get(pool: &SqlitePool, id: &str) -> Result<SessionRow> {
        let row = sqlx::query("SELECT * FROM sessions WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await?;
        Ok(from_row(row))
    }

    pub async fn list(pool: &SqlitePool, limit: i64) -> Result<Vec<SessionRow>> {
        let rows = sqlx::query("SELECT * FROM sessions ORDER BY updated_at DESC LIMIT ?")
            .bind(limit)
            .fetch_all(pool)
            .await?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn search(pool: &SqlitePool, query: &str, limit: i64) -> Result<Vec<SessionRow>> {
        let pattern = format!("%{query}%");
        let rows = sqlx::query(
            "SELECT * FROM sessions WHERE title LIKE ? OR system_prompt LIKE ? \
             ORDER BY updated_at DESC LIMIT ?",
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(limit)
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn rename(pool: &SqlitePool, id: &str, title: &str) -> Result<()> {
        let now = now_ts();
        sqlx::query("UPDATE sessions SET title = ?, updated_at = ? WHERE id = ?")
            .bind(title)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn touch(pool: &SqlitePool, id: &str) -> Result<()> {
        let _ = Utc::now();
        let now = now_ts();
        sqlx::query("UPDATE sessions SET updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM sessions WHERE id = ?")
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
    async fn crud_roundtrip() {
        let pool = crate::database::pool::in_memory_pool().await.unwrap();
        let s = SessionRepo::create(&pool, "s1", "Test", "openai", "gpt-4o", None)
            .await
            .unwrap();
        assert_eq!(s.title, "Test");

        SessionRepo::rename(&pool, "s1", "Renamed").await.unwrap();
        let s2 = SessionRepo::get(&pool, "s1").await.unwrap();
        assert_eq!(s2.title, "Renamed");

        let list = SessionRepo::list(&pool, 10).await.unwrap();
        assert_eq!(list.len(), 1);

        SessionRepo::delete(&pool, "s1").await.unwrap();
        let list2 = SessionRepo::list(&pool, 10).await.unwrap();
        assert!(list2.is_empty());
    }
}
