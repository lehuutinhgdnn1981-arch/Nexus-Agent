//! NEXUS — tool system.

pub mod context;
pub mod registry;
pub mod schema;
pub mod tool;

#[cfg(test)]
mod tests;

// Tool category modules
pub mod browser;
pub mod code;
pub mod file;
pub mod memory;
pub mod scheduler;
pub mod search;
pub mod shell;

pub use context::ToolContext;
pub use registry::ToolRegistry;
pub use schema::ToolSchema;
pub use tool::{Tool, ToolCall, ToolError, ToolResult};

/// Register toàn bộ default tools vào registry.
/// Gọi 1 lần lúc app startup từ `AppState::register_default_tools`.
pub fn register_all(registry: &ToolRegistry) {
    file::register_all(registry);
    shell::register_all(registry);
    code::register_all(registry);
    browser::register_all(registry);
    search::register_all(registry);
    memory::register_all(registry);
    scheduler::register_all(registry);
}
