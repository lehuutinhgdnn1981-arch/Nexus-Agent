//! Tool registry — quản lý đăng ký + lookup tool.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use tracing::info;

use crate::tools::schema::ToolSchema;
use crate::tools::tool::Tool;

/// Tool registry — thread-safe.
#[derive(Default)]
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Arc<dyn Tool>>>,
}

impl ToolRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register 1 tool.
    pub fn register<T: Tool + 'static>(&self, tool: T) {
        let name = tool.name().to_string();
        let arc: Arc<dyn Tool> = Arc::new(tool);
        let mut guard = self.tools.write();
        guard.insert(name.clone(), arc);
        info!(tool = %name, "tool registered");
    }

    /// Register tool từ Arc.
    pub fn register_arc(&self, name: impl Into<String>, tool: Arc<dyn Tool>) {
        let name = name.into();
        let mut guard = self.tools.write();
        guard.insert(name.clone(), tool);
        info!(tool = %name, "tool registered (arc)");
    }

    /// Lookup tool theo tên.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        let guard = self.tools.read();
        guard.get(name).cloned()
    }

    /// List tất cả tool schemas (cho LLM).
    #[must_use]
    pub fn all_schemas(&self) -> Vec<ToolSchema> {
        let guard = self.tools.read();
        guard
            .values()
            .map(|t| ToolSchema::new(t.name(), t.description(), t.schema()))
            .collect()
    }

    /// List tất cả tool names.
    #[must_use]
    pub fn list_names(&self) -> Vec<String> {
        let guard = self.tools.read();
        guard.keys().cloned().collect()
    }

    /// Count tools.
    #[must_use]
    pub fn len(&self) -> usize {
        let guard = self.tools.read();
        guard.len()
    }

    /// Empty?
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Auto-register toàn bộ default tools (gọi 1 lần lúc startup).
    pub fn register_defaults(&self) {
        // Sẽ gọi từ `tools::init()` ở Phase 5.
        // Phase 4: không có tool cụ thể nào.
    }
}
