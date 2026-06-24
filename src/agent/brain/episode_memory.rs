//! Episode memory — log (tool, input, outcome) để tránh lặp lại sai lầm.
//!
//! Khi agent thử tool X với input Y và fail, episode memory nhớ điều đó.
//! Lần sau khi agent định gọi tool X với input tương tự Y, recall episode và warn.
//!
//! Lưu trong SQLite table `episodes` (separate từ `memories` table).

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tracing::{debug, info, warn};

use crate::error::Result;
use crate::llm::provider::LLMProvider;
use crate::memory::cosine::cosine_similarity;
use crate::utils::{ids::new_uuid, time::now_ts};

/// 1 episode = 1 tool call outcome.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Episode {
    pub id: String,
    pub tool: String,
    pub input: serde_json::Value,
    /// Embedding của input (để similarity search).
    pub input_embedding: Vec<f32>,
    pub success: bool,
    pub error_message: Option<String>,
    pub session_id: Option<String>,
    pub created_at: i64,
}

/// Episode memory store — SQLite-backed.
pub struct EpisodeMemory {
    pool: SqlitePool,
    provider: Arc<dyn LLMProvider>,
    /// Similarity threshold để consider 2 episodes "same input".
    similarity_threshold: f32,
}

impl EpisodeMemory {
    pub fn new(pool: SqlitePool, provider: Arc<dyn LLMProvider>, similarity_threshold: f32) -> Self {
        Self {
            pool,
            provider,
            similarity_threshold,
        }
    }

    /// Initialize `episodes` table (gọi 1 lần khi app start).
    pub async fn init_schema(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS episodes (
                id           TEXT PRIMARY KEY,
                tool         TEXT NOT NULL,
                input        TEXT NOT NULL,
                embedding    BLOB NOT NULL,
                embedding_dim INTEGER NOT NULL,
                success      INTEGER NOT NULL,
                error_message TEXT,
                session_id   TEXT,
                created_at   INTEGER NOT NULL
            )",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_episodes_tool ON episodes(tool, success)",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Record 1 episode (sau khi tool call hoàn thành).
    pub async fn record(
        &self,
        tool: &str,
        input: &serde_json::Value,
        success: bool,
        error_message: Option<&str>,
        session_id: Option<&str>,
    ) -> Result<String> {
        let id = new_uuid();
        let input_str = serde_json::to_string(input).unwrap_or_default();
        let embedding = self.provider.embed(&format!("{tool}: {input_str}")).await.unwrap_or_default();
        let dim = embedding.len() as i64;
        let emb_bytes = embedding_to_bytes(&embedding);
        let now = now_ts();

        sqlx::query(
            "INSERT INTO episodes (id, tool, input, embedding, embedding_dim, success, error_message, session_id, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(tool)
        .bind(&input_str)
        .bind(&emb_bytes)
        .bind(dim)
        .bind(if success { 1_i64 } else { 0 })
        .bind(error_message)
        .bind(session_id)
        .bind(now)
        .execute(&self.pool)
        .await?;

        debug!(tool, success, "episode recorded");
        Ok(id)
    }

    /// Find similar failed episodes — nếu có, trả về error message để agent tránh lặp.
    pub async fn find_similar_failures(
        &self,
        tool: &str,
        input: &serde_json::Value,
    ) -> Result<Vec<SimilarFailure>> {
        let input_str = serde_json::to_string(input).unwrap_or_default();
        let query_embedding = self.provider.embed(&format!("{tool}: {input_str}")).await?;
        let dim = query_embedding.len() as i64;

        let rows = sqlx::query_as::<_, EpisodeRow>(
            "SELECT * FROM episodes WHERE tool = ? AND success = 0 AND embedding_dim = ?",
        )
        .bind(tool)
        .bind(dim)
        .fetch_all(&self.pool)
        .await?;

        let mut failures = Vec::new();
        for row in rows {
            let emb = bytes_to_embedding(&row.embedding);
            let sim = cosine_similarity(&query_embedding, &emb);
            if sim >= self.similarity_threshold {
                failures.push(SimilarFailure {
                    tool: row.tool,
                    input: serde_json::from_str(&row.input).unwrap_or(serde_json::Value::Null),
                    error_message: row.error_message.unwrap_or_default(),
                    similarity: sim,
                    age_seconds: now_ts() - row.created_at,
                });
            }
        }

        // Sort by similarity desc
        failures.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));

        if !failures.is_empty() {
            info!(
                tool,
                count = failures.len(),
                top_similarity = failures.first().map(|f| f.similarity).unwrap_or(0.0),
                "similar failed episodes found"
            );
        }

        Ok(failures)
    }

    /// Get success rate của 1 tool (cho analytics).
    pub async fn tool_success_rate(&self, tool: &str) -> Result<f32> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM episodes WHERE tool = ?")
            .bind(tool)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        if total == 0 {
            return Ok(1.0); // assume success if no data
        }

        let success_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM episodes WHERE tool = ? AND success = 1")
                .bind(tool)
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);

        Ok(success_count as f32 / total as f32)
    }
}

/// Similar failure (cho agent context).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimilarFailure {
    pub tool: String,
    pub input: serde_json::Value,
    pub error_message: String,
    pub similarity: f32,
    pub age_seconds: i64,
}

impl SimilarFailure {
    /// Format thành warning text cho LLM context.
    #[must_use]
    pub fn to_warning(&self) -> String {
        format!(
            "⚠️ Similar failure in past: tool `{}` with input `{}` failed with: `{}` (similarity: {:.2}, {} ago)",
            self.tool,
            self.input,
            self.error_message,
            self.similarity,
            humantime::format_duration(std::time::Duration::from_secs(self.age_seconds.unsigned_abs())),
        )
    }
}

/// SQLx row mapping.
#[derive(sqlx::FromRow)]
struct EpisodeRow {
    id: String,
    tool: String,
    input: String,
    embedding: Vec<u8>,
    embedding_dim: i64,
    success: i64,
    error_message: Option<String>,
    session_id: Option<String>,
    created_at: i64,
}

/// Convert Vec<f32> → little-endian byte blob.
#[must_use]
fn embedding_to_bytes(vec: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vec.len() * 4);
    for &v in vec {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    bytes
}

/// Convert little-endian byte blob → Vec<f32>.
#[must_use]
fn bytes_to_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| {
            let arr: [u8; 4] = chunk.try_into().unwrap_or([0, 0, 0, 0]);
            f32::from_le_bytes(arr)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MockProvider;

    #[tokio::test]
    async fn init_schema_creates_table() {
        let pool = crate::database::pool::in_memory_pool().await.unwrap();
        let mock = Arc::new(MockProvider::with_embedding(vec![0.1; 8]));
        let mem = EpisodeMemory::new(pool, mock, 0.85);
        mem.init_schema().await.unwrap();
        // Re-run should be idempotent
        mem.init_schema().await.unwrap();
    }

    #[tokio::test]
    async fn record_and_find_similar_failure() {
        let pool = crate::database::pool::in_memory_pool().await.unwrap();
        let mock = Arc::new(MockProvider::with_embedding(vec![0.5; 8]));
        let mem = EpisodeMemory::new(pool, mock, 0.85);
        mem.init_schema().await.unwrap();

        // Record a failure
        mem.record(
            "read_file",
            &serde_json::json!({"path": "missing.txt"}),
            false,
            Some("file not found"),
            Some("s1"),
        )
        .await
        .unwrap();

        // Find similar — should match because embedding is identical (mock returns same)
        let failures = mem
            .find_similar_failures("read_file", &serde_json::json!({"path": "missing.txt"}))
            .await
            .unwrap();
        assert_eq!(failures.len(), 1);
        assert!(failures[0].similarity >= 0.99);
        assert!(failures[0].error_message.contains("file not found"));
    }

    #[tokio::test]
    async fn record_success_and_compute_rate() {
        let pool = crate::database::pool::in_memory_pool().await.unwrap();
        let mock = Arc::new(MockProvider::with_embedding(vec![0.5; 8]));
        let mem = EpisodeMemory::new(pool, mock, 0.85);
        mem.init_schema().await.unwrap();

        mem.record("run_command", &serde_json::json!({"command": "echo hi"}), true, None, None)
            .await
            .unwrap();
        mem.record("run_command", &serde_json::json!({"command": "ls"}), true, None, None)
            .await
            .unwrap();
        mem.record("run_command", &serde_json::json!({"command": "false"}), false, Some("exit 1"), None)
            .await
            .unwrap();

        let rate = mem.tool_success_rate("run_command").await.unwrap();
        assert!((rate - 0.6667).abs() < 0.01);
    }

    #[tokio::test]
    async fn tool_success_rate_no_data_returns_one() {
        let pool = crate::database::pool::in_memory_pool().await.unwrap();
        let mock = Arc::new(MockProvider::with_embedding(vec![0.5; 8]));
        let mem = EpisodeMemory::new(pool, mock, 0.85);
        mem.init_schema().await.unwrap();

        let rate = mem.tool_success_rate("never_used").await.unwrap();
        assert_eq!(rate, 1.0);
    }

    #[test]
    fn similar_failure_warning_formats() {
        let f = SimilarFailure {
            tool: "read_file".into(),
            input: serde_json::json!({"path": "x"}),
            error_message: "not found".into(),
            similarity: 0.95,
            age_seconds: 3600,
        };
        let warning = f.to_warning();
        assert!(warning.contains("read_file"));
        assert!(warning.contains("not found"));
        assert!(warning.contains("0.95"));
    }
}
