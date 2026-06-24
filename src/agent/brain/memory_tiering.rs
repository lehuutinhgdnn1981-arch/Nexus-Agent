//! Memory tiering — Working / Archival / Recall.
//!
//! Inspired by MemGPT (letta-ai/letta).
//!
//! - **Working memory**: messages gần đây trong context window (đã có ở `ShortTermMemory`).
//! - **Archival memory**: long-term facts/preferences (đã có ở `LongTermMemory`).
//! - **Recall memory**: summary nén của conversations cũ, dùng để tiết kiệm context.
//!
//! Mô-đun này thêm:
//! - Background summarization loop — định kỳ summarize conversation → Recall memory.
//! - Decay — Archival memories cũ ít dùng → move to Recall (giảm weight).
//! - Context builder — build messages theo tier: [system, recall, archival_relevant, working].

use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::database::repositories::memory_repo::{MemoryRepo, MemoryRow};
use crate::error::Result;
use crate::llm::provider::LLMProvider;
use crate::llm::types::{ChatMessage, ChatRequest};
use crate::memory::model::MemoryCategory;
use crate::utils::{ids::new_uuid, time::now_ts};

/// Số messages tối đa trong working memory trước khi trigger summarization.
const SUMMARIZE_THRESHOLD: usize = 50;

/// Số ngày không dùng → Archival memory decay xuống Recall.
const DECAY_DAYS: i64 = 30;

/// Số use_count tối đa để được giữ ở Archival (dưới ngưỡng → decay).
const DECAY_USE_COUNT: i64 = 5;

/// Tier của memory.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryTier {
    /// Trong context window — messages gần đây (không lưu DB).
    Working,
    /// Facts/preferences recall qua embedding search (existing `memories` table).
    Archival,
    /// Summary nén — category = "recall".
    Recall,
}

impl MemoryTier {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Working => "working",
            Self::Archival => "archival",
            Self::Recall => "recall",
        }
    }
}

/// Config cho memory tiering.
#[derive(Clone, Debug)]
pub struct TieringConfig {
    pub summarize_threshold: usize,
    pub decay_days: i64,
    pub decay_use_count: i64,
    pub recall_model: Option<String>,
}

impl Default for TieringConfig {
    fn default() -> Self {
        Self {
            summarize_threshold: SUMMARIZE_THRESHOLD,
            decay_days: DECAY_DAYS,
            decay_use_count: DECAY_USE_COUNT,
            recall_model: None, // dùng default_model của agent
        }
    }
}

/// Memory tier manager — orchestrate summarization + decay.
pub struct MemoryTierManager {
    pool: SqlitePool,
    provider: Arc<dyn LLMProvider>,
    config: TieringConfig,
    /// Lock để tránh 2 summarization chạy song song.
    summarization_lock: Mutex<()>,
}

impl MemoryTierManager {
    pub fn new(pool: SqlitePool, provider: Arc<dyn LLMProvider>, config: TieringConfig) -> Self {
        Self {
            pool,
            provider,
            config,
            summarization_lock: Mutex::new(()),
        }
    }

    /// Kiểm tra xem cần summarize không (gọi sau mỗi turn).
    /// Trả về Some(summary) nếu đã summarize, None nếu chưa cần.
    pub async fn maybe_summarize(
        &self,
        session_id: &str,
        messages: &[ChatMessage],
    ) -> Result<Option<String>> {
        if messages.len() < self.config.summarize_threshold {
            return Ok(None);
        }

        let _guard = self.summarization_lock.lock().await;

        // Build messages để gửi LLM — system prompt + N messages gần nhất
        let messages_to_summarize = &messages[messages.len().saturating_sub(self.config.summarize_threshold)..];
        let mut summarize_messages = Vec::with_capacity(messages_to_summarize.len() + 1);
        summarize_messages.push(ChatMessage::system(
            "Summarize the following conversation in 5-10 bullet points. \
             Focus on: decisions made, files touched, errors encountered, user preferences learned. \
             Be concise and factual. Do not include pleasantries.",
        ));
        for msg in messages_to_summarize {
            let role = match msg.role {
                crate::llm::types::MessageRole::System => continue,
                crate::llm::types::MessageRole::User => "user",
                crate::llm::types::MessageRole::Assistant => "assistant",
                crate::llm::types::MessageRole::Tool => "tool",
            };
            summarize_messages.push(ChatMessage::user(format!("[{role}]: {}", msg.content)));
        }

        let model = self.config.recall_model.as_deref().unwrap_or("gpt-4o-mini");
        let req = ChatRequest::new(model, summarize_messages);
        let resp = match self.provider.chat(req).await {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, "summarization LLM call failed");
                return Err(e.into());
            }
        };

        let summary = resp.content;
        debug!(session_id, summary_len = summary.len(), "conversation summarized");

        // Lưu vào DB với category = "recall"
        let id = new_uuid();
        let now = now_ts();
        let tags = serde_json::to_string(&["recall".to_string(), "summary".to_string()])
            .unwrap_or_else(|_| "[]".into());

        // Embedding cho summary (để recall được)
        let embedding = self.provider.embed(&summary).await.unwrap_or_default();
        let dim = embedding.len() as i64;
        let embedding_bytes = crate::database::repositories::memory_repo::embedding_to_bytes(&embedding);

        let row = MemoryRow {
            id,
            content: summary.clone(),
            category: MemoryCategory::Note.as_str().to_string(), // Recall dùng category "note" + tag "recall"
            tags,
            embedding: embedding_bytes,
            embedding_dim: dim,
            session_id: Some(session_id.to_string()),
            created_at: now,
            last_used_at: now,
            use_count: 0,
        };
        MemoryRepo::insert(&self.pool, &row).await?;
        info!(session_id, recall_id = %row.id, "recall memory created");

        Ok(Some(summary))
    }

    /// Decay — move Archival memories ít dùng sang Recall (mark with tag).
    /// Gọi định kỳ (vd mỗi giờ) hoặc khi app idle.
    pub async fn run_decay(&self) -> Result<usize> {
        let cutoff_ts = Utc::now().timestamp() - (self.config.decay_days * 86_400);
        let candidates = MemoryRepo::list(&self.pool, 1000).await?;

        let mut decayed = 0;
        for row in candidates {
            // Skip memories đã là recall
            if row.tags.contains("\"recall\"") {
                continue;
            }
            if row.last_used_at < cutoff_ts && row.use_count < self.config.decay_use_count {
                // Mark as decayed — add "recall" tag (giữ nguyên content)
                let mut tags: Vec<String> = serde_json::from_str(&row.tags).unwrap_or_default();
                if !tags.contains(&"recall".to_string()) {
                    tags.push("recall".to_string());
                }
                let new_tags = serde_json::to_string(&tags).unwrap_or_default();
                MemoryRepo::merge_tags(&self.pool, &row.id, &new_tags).await?;
                decayed += 1;
            }
        }

        if decayed > 0 {
            info!(decayed, "memories decayed to recall tier");
        }
        Ok(decayed)
    }

    /// Build context messages theo tiering pattern.
    /// Trả về: [recall_summary, relevant_archival, working_messages...]
    /// Caller thêm system prompt lên đầu.
    pub async fn build_context(
        &self,
        session_id: &str,
        working_messages: &[ChatMessage],
        query: &str,
        top_k_archival: usize,
    ) -> Result<Vec<ChatMessage>> {
        let mut context = Vec::new();

        // 1. Recall — summary gần nhất của session này
        let recall_memories = MemoryRepo::list_by_category(&self.pool, "note", 10).await?;
        let session_recall: Vec<_> = recall_memories
            .into_iter()
            .filter(|m| {
                m.session_id.as_deref() == Some(session_id) && m.tags.contains("\"recall\"")
            })
            .collect();
        if !session_recall.is_empty() {
            let recall_text = session_recall
                .iter()
                .map(|m| m.content.clone())
                .collect::<Vec<_>>()
                .join("\n---\n");
            context.push(ChatMessage::system(format!(
                "## Recall memory (summary of previous conversations):\n{recall_text}"
            )));
        }

        // 2. Archival — relevant facts via embedding search (skip recall-tagged)
        let query_embedding = self.provider.embed(query).await.unwrap_or_default();
        if !query_embedding.is_empty() {
            let dim = query_embedding.len() as i64;
            let all_archival = MemoryRepo::all_embeddings(&self.pool, dim).await?;
            let mut scored: Vec<_> = all_archival
                .into_iter()
                .filter(|m| !m.tags.contains("\"recall\""))
                .map(|m| {
                    let emb = crate::database::repositories::memory_repo::bytes_to_embedding(&m.embedding);
                    let sim = crate::memory::cosine::cosine_similarity(&query_embedding, &emb);
                    (sim, m)
                })
                .collect();
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            let top: Vec<_> = scored.into_iter().take(top_k_archival).collect();

            if !top.is_empty() {
                let archival_text = top
                    .iter()
                    .map(|(sim, m)| {
                        let _ = sim;
                        format!("- [{}] {}", m.category, m.content)
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                context.push(ChatMessage::system(format!(
                    "## Relevant archival memories:\n{archival_text}"
                )));

                // Bump usage cho recalled memories
                for (_, m) in &top {
                    let _ = MemoryRepo::bump_usage(&self.pool, &m.id).await;
                }
            }
        }

        // 3. Working memory
        context.extend(working_messages.iter().cloned());

        Ok(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MockProvider;

    #[test]
    fn tier_as_str() {
        assert_eq!(MemoryTier::Working.as_str(), "working");
        assert_eq!(MemoryTier::Archival.as_str(), "archival");
        assert_eq!(MemoryTier::Recall.as_str(), "recall");
    }

    #[test]
    fn config_default_uses_constants() {
        let cfg = TieringConfig::default();
        assert_eq!(cfg.summarize_threshold, SUMMARIZE_THRESHOLD);
        assert_eq!(cfg.decay_days, DECAY_DAYS);
        assert_eq!(cfg.decay_use_count, DECAY_USE_COUNT);
    }

    #[tokio::test]
    async fn maybe_summarize_skips_when_under_threshold() {
        let pool = crate::database::pool::in_memory_pool().await.unwrap();
        let mock = Arc::new(MockProvider::with_embedding(vec![0.1; 8]));
        let mgr = MemoryTierManager::new(pool, mock, TieringConfig::default());

        let msgs = vec![ChatMessage::user("hi")];
        let result = mgr.maybe_summarize("s1", &msgs).await.unwrap();
        assert!(result.is_none());
    }
}
