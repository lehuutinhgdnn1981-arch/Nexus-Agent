//! Memory tools — wrappers around MemoryStore.

pub mod delete;
pub mod recall;
pub mod save;

pub fn register_all(registry: &crate::tools::registry::ToolRegistry) {
    registry.register(save::MemorySaveTool);
    registry.register(recall::MemoryRecallTool);
    registry.register(delete::MemoryDeleteTool);
}
