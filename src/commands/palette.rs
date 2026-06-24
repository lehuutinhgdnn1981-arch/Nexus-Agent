//! Command Palette IPC commands — search everything (sessions, memories, tools, jobs).
//!
//! Returns unified results cho frontend palette. Fuzzy match simple (substring + scored).

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

use super::{IpcError, IpcResult};

/// Một entry trong palette results.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PaletteItem {
    Session {
        id: String,
        title: String,
        provider: String,
        updated_at: i64,
        score: f64,
    },
    Memory {
        id: String,
        content: String,
        category: String,
        created_at: i64,
        score: f64,
    },
    Tool {
        name: String,
        description: String,
        permission: String,
        score: f64,
    },
    ScheduledJob {
        id: String,
        message: String,
        enabled: bool,
        score: f64,
    },
    QuickAction {
        id: String,
        title: String,
        description: String,
        icon: String,
        score: f64,
    },
}

impl PaletteItem {
    fn score(&self) -> f64 {
        match self {
            Self::Session { score, .. }
            | Self::Memory { score, .. }
            | Self::Tool { score, .. }
            | Self::ScheduledJob { score, .. }
            | Self::QuickAction { score, .. } => *score,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PaletteSearchInput {
    pub query: String,
    /// Giới hạn số results per category.
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    5
}

/// Quick actions mặc định — luôn available.
fn default_quick_actions(query: &str) -> Vec<PaletteItem> {
    let actions = [
        ("new_session", "New session", "Create a new chat session", "✚"),
        ("search_web", "Search web", &format!("Search the web for: {query}"), "🔍"),
        ("remember", "Remember", &format!("Save a memory: {query}"), "🧠"),
        ("schedule", "Schedule", &format!("Schedule a reminder: {query}"), "⏰"),
        ("open_settings", "Open settings", "Configure providers and security", "⚙"),
        ("clear_chat", "Clear chat", "Clear current session messages", "🗑"),
    ];

    actions
        .iter()
        .map(|(id, title, desc, icon)| {
            let score = if query.is_empty() {
                0.5
            } else {
                fuzzy_score(query, title)
            };
            PaletteItem::QuickAction {
                id: id.to_string(),
                title: title.to_string(),
                description: desc.to_string(),
                icon: icon.to_string(),
                score,
            }
        })
        .collect()
}

/// Fuzzy score — đơn giản. Trả về 0.0-1.0.
fn fuzzy_score(query: &str, target: &str) -> f64 {
    if query.is_empty() {
        return 0.5;
    }
    let q = query.to_lowercase();
    let t = target.to_lowercase();

    if t.contains(&q) {
        // Substring match — higher score for shorter targets (more relevant)
        let len_bonus = 1.0 / (t.len() as f64).sqrt();
        return 0.8 + len_bonus * 0.2;
    }

    // Check if all query chars appear in order (subsequence)
    let mut q_chars = q.chars().peekable();
    for c in t.chars() {
        if q_chars.peek() == Some(&c) {
            q_chars.next();
        }
    }
    if q_chars.peek().is_none() {
        return 0.6;
    }

    // Word-by-word match
    let q_words: Vec<&str> = q.split_whitespace().collect();
    if !q_words.is_empty() && q_words.iter().all(|w| t.contains(w)) {
        return 0.7;
    }

    0.0
}

#[tauri::command]
pub async fn palette_search(
    state: State<'_, Arc<AppState>>,
    input: PaletteSearchInput,
) -> IpcResult<Vec<PaletteItem>> {
    let query = input.query.trim();
    let limit = input.limit.max(1).min(20);
    let mut results: Vec<PaletteItem> = Vec::new();

    // 1. Quick actions (always available)
    results.extend(default_quick_actions(query));

    // 2. Sessions
    let sessions = crate::database::repositories::session_repo::SessionRepo::list(&state.pool, 100)
        .await
        .map_err(IpcError::from)?;
    for s in sessions {
        let score = fuzzy_score(query, &s.title).max(fuzzy_score(query, &s.id));
        if score > 0.0 || query.is_empty() {
            results.push(PaletteItem::Session {
                id: s.id,
                title: s.title,
                provider: s.provider,
                updated_at: s.updated_at,
                score,
            });
        }
    }

    // 3. Memories (long-term) — skip if query empty (too many)
    if !query.is_empty() {
        let memories = crate::database::repositories::memory_repo::MemoryRepo::list(&state.pool, 100)
            .await
            .map_err(IpcError::from)?;
        for m in memories {
            let score = fuzzy_score(query, &m.content);
            if score > 0.0 {
                results.push(PaletteItem::Memory {
                    id: m.id,
                    content: m.content.chars().take(200).collect(),
                    category: m.category,
                    created_at: m.created_at,
                    score,
                });
            }
        }
    }

    // 4. Tools
    let schemas = state.tool_registry.all_schemas();
    for schema in schemas {
        if let Some(tool) = state.tool_registry.get(&schema.name) {
            let score = fuzzy_score(query, &schema.name).max(fuzzy_score(query, &schema.description));
            if score > 0.0 || query.is_empty() {
                results.push(PaletteItem::Tool {
                    name: schema.name,
                    description: schema.description,
                    permission: tool.permission().label().to_string(),
                    score,
                });
            }
        }
    }

    // 5. Scheduled jobs
    let jobs = crate::database::repositories::task_repo::TaskRepo::list_all(&state.pool)
        .await
        .map_err(IpcError::from)?;
    for j in jobs {
        let score = fuzzy_score(query, &j.payload);
        if score > 0.0 || query.is_empty() {
            results.push(PaletteItem::ScheduledJob {
                id: j.id,
                message: j.payload.chars().take(200).collect(),
                enabled: j.enabled,
                score,
            });
        }
    }

    // Sort by score desc, take top N per category
    results.sort_by(|a, b| b.score().partial_cmp(&a.score()).unwrap_or(std::cmp::Ordering::Equal));

    // Limit total results
    results.truncate(limit * 5);

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_score_exact_match() {
        let s = fuzzy_score("hello", "Hello World");
        assert!(s > 0.8);
    }

    #[test]
    fn fuzzy_score_subsequence() {
        let s = fuzzy_score("hlo", "Hello");
        assert!(s > 0.5);
    }

    #[test]
    fn fuzzy_score_no_match() {
        let s = fuzzy_score("xyz", "Hello");
        assert_eq!(s, 0.0);
    }

    #[test]
    fn fuzzy_score_word_match() {
        let s = fuzzy_score("hello world", "Hello beautiful World");
        assert!(s > 0.6);
    }

    #[test]
    fn fuzzy_score_empty_query() {
        let s = fuzzy_score("", "anything");
        assert_eq!(s, 0.5);
    }
}
