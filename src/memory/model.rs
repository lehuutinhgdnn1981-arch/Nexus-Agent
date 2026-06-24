//! Memory domain model.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Category của memory (ràng buộc enum, không phải free string).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCategory {
    /// Sự thật khách quan.
    Fact,
    /// Sở thích / cấu hình user.
    Preference,
    /// Task / todo.
    Task,
    /// Note tự do.
    Note,
}

impl MemoryCategory {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fact => "fact",
            Self::Preference => "preference",
            Self::Task => "task",
            Self::Note => "note",
        }
    }

    #[must_use]
    pub fn from_str(s: &str) -> Self {
        match s {
            "fact" => Self::Fact,
            "preference" => Self::Preference,
            "task" => Self::Task,
            _ => Self::Note,
        }
    }
}

/// Một entry memory.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub category: MemoryCategory,
    pub tags: Vec<String>,
    pub embedding: Vec<f32>,
    pub session_id: Option<String>,
    pub created_at: i64,
    pub last_used_at: i64,
    pub use_count: u32,
}

/// Query để recall memory.
#[derive(Clone, Debug)]
pub struct MemoryQuery {
    pub text: String,
    pub top_k: u32,
    pub category: Option<MemoryCategory>,
    pub min_similarity: f32,
}

impl MemoryQuery {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            top_k: 5,
            category: None,
            min_similarity: 0.0,
        }
    }
}
