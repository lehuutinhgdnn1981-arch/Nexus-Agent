//! Message repository.

use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::{error::Result, utils::time::now_ts};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRow {
    pub id: String,
    pub session_id: String,
    pub role: String,                       // 'user' | 'assistant' | 'system' | 'tool'
    pub content: String,
    pub tool_calls: Option<String>,         // JSON
    pub tool_results: Option<String>,       // JSON
    pub created_at: i64,
}

fn from_row(row: SqliteRow) -> MessageRow {
    MessageRow {
        id: row.get("id"),
        session_id: row.get("session_id"),
        role: row.get("role"),
        content: row.get("content"),
        tool_calls: row.get("tool_calls"),
        tool_results: row.get("tool_results"),
        created_at: row.get("created_at"),
    }
}

pub struct MessageRepo;

impl MessageRepo {
    pub async fn append(
        pool: &SqlitePool,
        id: &str,
        session_id: &str,
        role: &str,
        content: &str,
        tool_calls: Option<&serde_json::Value>,
        tool_results: Option<&serde_json::Value>,
    ) -> Result<MessageRow> {
        let now = now_ts();
        let tc_json = tool_calls.map(|v| v.to_string());
        let tr_json = tool_results.map(|v| v.to_string());

        sqlx::query(
            "INSERT INTO messages (id, session_id, role, content, tool_calls, tool_results, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(session_id)
        .bind(role)
        .bind(content)
        .bind(&tc_json)
        .bind(&tr_json)
        .bind(now)
        .execute(pool)
        .await?;

        // bump session.updated_at
        crate::database::repositories::session_repo::SessionRepo::touch(pool, session_id).await?;

        Self::get(pool, id).await
    }

    pub async fn get(pool: &SqlitePool, id: &str) -> Result<MessageRow> {
        let row = sqlx::query("SELECT * FROM messages WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await?;
        Ok(from_row(row))
    }

    pub async fn list_by_session(pool: &SqlitePool, session_id: &str) -> Result<Vec<MessageRow>> {
        let rows = sqlx::query(
            "SELECT * FROM messages WHERE session_id = ? ORDER BY created_at ASC",
        )
        .bind(session_id)
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn delete_by_session(pool: &SqlitePool, session_id: &str) -> Result<u64> {
        let res = sqlx::query("DELETE FROM messages WHERE session_id = ?")
            .bind(session_id)
            .execute(pool)
            .await?;
        Ok(res.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn append_and_list() {
        let pool = crate::database::pool::in_memory_pool().await.unwrap();
        crate::database::repositories::session_repo::SessionRepo::create(
            &pool, "s1", "T", "openai", "gpt-4o", None,
        )
        .await
        .unwrap();

        MessageRepo::append(&pool, "m1", "s1", "user", "Hello", None, None)
            .await
            .unwrap();
        MessageRepo::append(&pool, "m2", "s1", "assistant", "Hi there", None, None)
            .await
            .unwrap();

        let list = MessageRepo::list_by_session(&pool, "s1").await.unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].role, "user");
        assert_eq!(list[1].role, "assistant");
    }
}
