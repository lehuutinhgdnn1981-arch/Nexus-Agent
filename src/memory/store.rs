//! Memory store — kết hợp short-term (per session) + long-term (global).

use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;
use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::error::Result;
use crate::llm::types::ChatMessage;
use crate::memory::embedding::EmbeddingClient;
use crate::memory::long_term::LongTermMemory;
use crate::memory::model::{MemoryCategory, MemoryEntry, MemoryQuery};
use crate::memory::short_term::ShortTermMemory;

/// Combined memory store.
pub struct MemoryStore {
    /// Per-session short-term buffer.
    short_term: DashMap<String, RwLock<ShortTermMemory>>,
    /// Global long-term store (SQLite).
    long_term: Arc<LongTermMemory>,
}

impl MemoryStore {
    pub fn new(
        pool: SqlitePool,
        embedding_client: Arc<EmbeddingClient>,
        dedup_threshold: f32,
    ) -> Self {
        let long_term = Arc::new(LongTermMemory::new(pool, embedding_client, dedup_threshold));
        Self {
            short_term: DashMap::new(),
            long_term,
        }
    }

    // === Short-term ops ===

    pub async fn push_short_term(&self, session_id: &str, msg: ChatMessage) {
        let entry = self
            .short_term
            .entry(session_id.to_string())
            .or_insert_with(|| RwLock::new(ShortTermMemory::new()));
        entry.write().await.push(msg);
    }

    pub async fn short_term_all(&self, session_id: &str) -> Vec<ChatMessage> {
        if let Some(entry) = self.short_term.get(session_id) {
            entry.read().await.all()
        } else {
            Vec::new()
        }
    }

    pub async fn short_term_recent(&self, session_id: &str, n: usize) -> Vec<ChatMessage> {
        if let Some(entry) = self.short_term.get(session_id) {
            entry.read().await.recent(n)
        } else {
            Vec::new()
        }
    }

    pub async fn clear_short_term(&self, session_id: &str) {
        if let Some(entry) = self.short_term.get(session_id) {
            entry.write().await.clear();
        }
    }

    pub fn drop_session(&self, session_id: &str) {
        self.short_term.remove(session_id);
    }

    // === Long-term ops ===

    pub async fn save_long_term(
        &self,
        content: &str,
        category: MemoryCategory,
        tags: Vec<String>,
        session_id: Option<&str>,
    ) -> Result<String> {
        self.long_term.save(content, category, tags, session_id).await
    }

    pub async fn recall(&self, query: &MemoryQuery) -> Result<Vec<MemoryEntry>> {
        self.long_term.recall(query).await
    }

    pub async fn list_recent(&self, limit: i64) -> Result<Vec<MemoryEntry>> {
        self.long_term.list_recent(limit).await
    }

    pub async fn list_by_category(
        &self,
        category: MemoryCategory,
        limit: i64,
    ) -> Result<Vec<MemoryEntry>> {
        self.long_term.list_by_category(category, limit).await
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        self.long_term.delete(id).await
    }

    /// Snapshot short-term as HashMap (cho debug / state export).
    #[must_use]
    pub fn snapshot_session_ids(&self) -> Vec<String> {
        self.short_term
            .iter()
            .map(|e| e.key().clone())
            .collect()
    }

    /// Lấy reference Arc<LongTermMemory> (cho tool implementations).
    #[must_use]
    pub fn long_term(&self) -> Arc<LongTermMemory> {
        Arc::clone(&self.long_term)
    }

    /// Trả về HashMap<&str, usize> — session_id → message count.
    #[must_use]
    pub fn short_term_counts(&self) -> HashMap<String, usize> {
        self.short_term
            .iter()
            .map(|e| (e.key().clone(), e.value().try_read().map(|r| r.len()).unwrap_or(0)))
            .collect()
    }
}
