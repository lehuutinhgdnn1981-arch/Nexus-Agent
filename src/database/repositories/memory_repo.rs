//! Memory repository (long-term memory với embeddings).

use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::{error::Result, utils::time::now_ts};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRow {
    pub id: String,
    pub content: String,
    pub category: String,
    pub tags: String,                       // JSON array
    pub embedding: Vec<u8>,                 // f32 LE bytes
    pub embedding_dim: i64,
    pub session_id: Option<String>,
    pub created_at: i64,
    pub last_used_at: i64,
    pub use_count: i64,
}

fn from_row(row: SqliteRow) -> MemoryRow {
    let blob: Vec<u8> = row.get("embedding");
    MemoryRow {
        id: row.get("id"),
        content: row.get("content"),
        category: row.get("category"),
        tags: row.get("tags"),
        embedding: blob,
        embedding_dim: row.get("embedding_dim"),
        session_id: row.get("session_id"),
        created_at: row.get("created_at"),
        last_used_at: row.get("last_used_at"),
        use_count: row.get("use_count"),
    }
}

/// Convert Vec<f32> → little-endian byte blob.
#[must_use]
pub fn embedding_to_bytes(vec: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vec.len() * 4);
    for &v in vec {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    bytes
}

/// Convert little-endian byte blob → Vec<f32>.
#[must_use]
pub fn bytes_to_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| {
            let arr: [u8; 4] = chunk.try_into().unwrap_or([0, 0, 0, 0]);
            f32::from_le_bytes(arr)
        })
        .collect()
}

pub struct MemoryRepo;

impl MemoryRepo {
    pub async fn insert(pool: &SqlitePool, m: &MemoryRow) -> Result<()> {
        let now = now_ts();
        sqlx::query(
            "INSERT INTO memories \
             (id, content, category, tags, embedding, embedding_dim, session_id, created_at, last_used_at, use_count) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&m.id)
        .bind(&m.content)
        .bind(&m.category)
        .bind(&m.tags)
        .bind(&m.embedding)
        .bind(m.embedding_dim)
        .bind(&m.session_id)
        .bind(now)
        .bind(now)
        .bind(m.use_count)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get(pool: &SqlitePool, id: &str) -> Result<Option<MemoryRow>> {
        let row = sqlx::query("SELECT * FROM memories WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        Ok(row.map(from_row))
    }

    pub async fn list(pool: &SqlitePool, limit: i64) -> Result<Vec<MemoryRow>> {
        let rows = sqlx::query("SELECT * FROM memories ORDER BY last_used_at DESC LIMIT ?")
            .bind(limit)
            .fetch_all(pool)
            .await?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn list_by_category(
        pool: &SqlitePool,
        category: &str,
        limit: i64,
    ) -> Result<Vec<MemoryRow>> {
        let rows =
            sqlx::query("SELECT * FROM memories WHERE category = ? ORDER BY last_used_at DESC LIMIT ?")
                .bind(category)
                .bind(limit)
                .fetch_all(pool)
                .await?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Lấy tất cả memory có embedding_dim = `dim` (để tính cosine similarity brute-force).
    pub async fn all_embeddings(pool: &SqlitePool, dim: i64) -> Result<Vec<MemoryRow>> {
        let rows = sqlx::query("SELECT * FROM memories WHERE embedding_dim = ?")
            .bind(dim)
            .fetch_all(pool)
            .await?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn bump_usage(pool: &SqlitePool, id: &str) -> Result<()> {
        let now = now_ts();
        sqlx::query(
            "UPDATE memories SET use_count = use_count + 1, last_used_at = ? WHERE id = ?",
        )
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn merge_tags(pool: &SqlitePool, id: &str, new_tags: &str) -> Result<()> {
        sqlx::query("UPDATE memories SET tags = ? WHERE id = ?")
            .bind(new_tags)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM memories WHERE id = ?")
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
    async fn embedding_roundtrip() {
        let v = vec![0.1_f32, 0.2, 0.3, 0.4];
        let bytes = embedding_to_bytes(&v);
        assert_eq!(bytes.len(), 16);
        let decoded = bytes_to_embedding(&bytes);
        assert_eq!(v, decoded);
    }

    #[tokio::test]
    async fn insert_and_get() {
        let pool = crate::database::pool::in_memory_pool().await.unwrap();
        let emb = vec![0.5_f32; 8];
        let m = MemoryRow {
            id: "m1".into(),
            content: "test memory".into(),
            category: "fact".into(),
            tags: "[\"test\"]".into(),
            embedding: embedding_to_bytes(&emb),
            embedding_dim: 8,
            session_id: Some("s1".into()),
            created_at: now_ts(),
            last_used_at: now_ts(),
            use_count: 0,
        };
        MemoryRepo::insert(&pool, &m).await.unwrap();
        let got = MemoryRepo::get(&pool, "m1").await.unwrap().unwrap();
        assert_eq!(got.content, "test memory");
        assert_eq!(bytes_to_embedding(&got.embedding), emb);
    }
}
