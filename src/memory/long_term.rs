//! Long-term memory — SQLite-backed store with embeddings.

use std::sync::Arc;

use sqlx::SqlitePool;
use tracing::info;

use crate::database::repositories::memory_repo::{
    bytes_to_embedding, embedding_to_bytes, MemoryRepo, MemoryRow,
};
use crate::error::Result;
use crate::memory::cosine::cosine_similarity;
use crate::memory::embedding::EmbeddingClient;
use crate::memory::model::{MemoryCategory, MemoryEntry, MemoryQuery};
use crate::utils::time::now_ts;

/// Long-term memory store.
pub struct LongTermMemory {
    pool: SqlitePool,
    embedding_client: Arc<EmbeddingClient>,
    dedup_threshold: f32,
}

impl LongTermMemory {
    pub fn new(pool: SqlitePool, embedding_client: Arc<EmbeddingClient>, dedup_threshold: f32) -> Self {
        Self {
            pool,
            embedding_client,
            dedup_threshold,
        }
    }

    /// Save một entry. Trả về id của entry (mới hoặc đã có nếu dedup).
    ///
    /// Flow:
    /// 1. Sinh embedding cho content.
    /// 2. Tìm entry có similarity >= dedup_threshold.
    ///    - Nếu có: merge tags + bump use_count + last_used_at, return id cũ.
    ///    - Nếu không: insert entry mới.
    pub async fn save(
        &self,
        content: &str,
        category: MemoryCategory,
        mut tags: Vec<String>,
        session_id: Option<&str>,
    ) -> Result<String> {
        let embedding = self.embedding_client.embed(content).await?;
        let dim = embedding.len() as i64;

        // Search existing for dedup
        let existing = MemoryRepo::all_embeddings(&self.pool, dim).await?;
        for row in &existing {
            let existing_emb = bytes_to_embedding(&row.embedding);
            let sim = cosine_similarity(&embedding, &existing_emb);
            if sim >= self.dedup_threshold {
                // Dedup: merge tags + bump usage
                let mut existing_tags: Vec<String> =
                    serde_json::from_str(&row.tags).unwrap_or_default();
                for t in &tags {
                    if !existing_tags.contains(t) {
                        existing_tags.push(t.clone());
                    }
                }
                let merged_json = serde_json::to_string(&existing_tags).unwrap_or_default();
                MemoryRepo::merge_tags(&self.pool, &row.id, &merged_json).await?;
                MemoryRepo::bump_usage(&self.pool, &row.id).await?;
                info!(id = %row.id, similarity = sim, "memory deduped");
                // Reconstruct tags for return value
                tags = existing_tags;
                let _ = tags; // silence
                return Ok(row.id.clone());
            }
        }

        // Insert new
        let id = crate::utils::ids::new_uuid();
        let now = now_ts();
        let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".into());
        let row = MemoryRow {
            id: id.clone(),
            content: content.to_string(),
            category: category.as_str().to_string(),
            tags: tags_json,
            embedding: embedding_to_bytes(&embedding),
            embedding_dim: dim,
            session_id: session_id.map(String::from),
            created_at: now,
            last_used_at: now,
            use_count: 0,
        };
        MemoryRepo::insert(&self.pool, &row).await?;
        info!(id = %id, dim = dim, "memory inserted");
        Ok(id)
    }

    /// Recall top-K memory tương tự query text.
    pub async fn recall(&self, query: &MemoryQuery) -> Result<Vec<MemoryEntry>> {
        let q_emb = self.embedding_client.embed(&query.text).await?;
        let dim = q_emb.len() as i64;
        let candidates = MemoryRepo::all_embeddings(&self.pool, dim).await?;

        let mut scored: Vec<(f32, MemoryRow)> = candidates
            .into_iter()
            .filter(|row| {
                query
                    .category
                    .map(|c| row.category == c.as_str())
                    .unwrap_or(true)
            })
            .map(|row| {
                let emb = bytes_to_embedding(&row.embedding);
                let sim = cosine_similarity(&q_emb, &emb);
                (sim, row)
            })
            .filter(|(sim, _)| *sim >= query.min_similarity)
            .collect();

        // Sort by similarity desc
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(query.top_k as usize);

        // Bump usage for recalled entries
        for (sim, row) in &scored {
            let _ = sim;
            MemoryRepo::bump_usage(&self.pool, &row.id).await.ok();
        }

        Ok(scored
            .into_iter()
            .map(|(sim, row)| MemoryEntry {
                id: row.id,
                content: row.content,
                category: MemoryCategory::from_str(&row.category),
                tags: serde_json::from_str(&row.tags).unwrap_or_default(),
                embedding: bytes_to_embedding(&row.embedding),
                session_id: row.session_id,
                created_at: row.created_at,
                last_used_at: row.last_used_at,
                use_count: row.use_count as u32,
            })
            .map(|mut e| {
                // store similarity in use_count temporarily? No — return as-is.
                let _ = &mut e;
                e
            })
            .collect())
    }

    /// List memory gần đây (không cần embedding search).
    pub async fn list_recent(&self, limit: i64) -> Result<Vec<MemoryEntry>> {
        let rows = MemoryRepo::list(&self.pool, limit).await?;
        Ok(rows.into_iter().map(row_to_entry).collect())
    }

    /// List memory theo category.
    pub async fn list_by_category(
        &self,
        category: MemoryCategory,
        limit: i64,
    ) -> Result<Vec<MemoryEntry>> {
        let rows = MemoryRepo::list_by_category(&self.pool, category.as_str(), limit).await?;
        Ok(rows.into_iter().map(row_to_entry).collect())
    }

    /// Delete memory by id.
    pub async fn delete(&self, id: &str) -> Result<()> {
        MemoryRepo::delete(&self.pool, id).await?;
        Ok(())
    }
}

fn row_to_entry(row: MemoryRow) -> MemoryEntry {
    MemoryEntry {
        id: row.id,
        content: row.content,
        category: MemoryCategory::from_str(&row.category),
        tags: serde_json::from_str(&row.tags).unwrap_or_default(),
        embedding: bytes_to_embedding(&row.embedding),
        session_id: row.session_id,
        created_at: row.created_at,
        last_used_at: row.last_used_at,
        use_count: row.use_count as u32,
    }
}
