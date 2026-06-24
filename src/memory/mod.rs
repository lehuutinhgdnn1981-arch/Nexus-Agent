//! NEXUS — memory system.

pub mod cosine;
pub mod embedding;
pub mod long_term;
pub mod model;
pub mod short_term;
pub mod store;

pub use embedding::EmbeddingClient;
pub use long_term::LongTermMemory;
pub use model::{MemoryCategory, MemoryEntry, MemoryQuery};
pub use short_term::ShortTermMemory;
pub use store::MemoryStore;
