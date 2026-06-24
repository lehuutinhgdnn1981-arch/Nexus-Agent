//! NEXUS brain — orchestration của 7 nâng cấp Agent.
//!
//! Modules:
//! - [`memory_tiering`] — Working/Archival/Recall tiering + summarization + decay.
//! - [`context_manager`] — token counting + compression.
//! - [`planner`] — Plan-and-execute pattern.
//! - [`reflection`] — self-critique on tool failure.
//! - [`tool_selector`] — RAG-style dynamic tool subset.
//! - [`episode_memory`] — avoid repeating mistakes.
//! - [`sub_agents`] — multi-agent handoff.
//!
//! [`Brain`] struct kết hợp tất cả, dùng trong `Agent::run`.

pub mod context_manager;
pub mod episode_memory;
pub mod memory_tiering;
pub mod planner;
pub mod reflection;
pub mod sub_agents;
pub mod tool_selector;

pub use context_manager::{ContextConfig, ContextManager};
pub use episode_memory::{EpisodeMemory, SimilarFailure};
pub use memory_tiering::{MemoryTier, MemoryTierManager, TieringConfig};
pub use planner::{generate_plan, should_plan, AgentPlan, PlanStep, PlanStepStatus};
pub use reflection::{reflect_on_failure, ReflectionResult};
pub use sub_agents::{
    default_sub_agents, detect_handoff, parse_handoff_request, sub_agent_schemas,
    sub_agent_system_prompt, SubAgent,
};
pub use tool_selector::DynamicToolSelector;

use std::sync::Arc;

use crate::error::Result;
use crate::llm::provider::LLMProvider;
use crate::llm::types::ChatMessage;
use crate::tools::registry::ToolRegistry;
use crate::tools::schema::ToolSchema;

/// Brain — orchestrates all 7 upgrade modules.
///
/// Created once per `Agent` instance. Holds references to:
/// - Provider (cho summarization, reflection, planning)
/// - Memory tier manager
/// - Context manager
/// - Tool selector
/// - Episode memory
pub struct Brain {
    pub provider: Arc<dyn LLMProvider>,
    pub memory_tiering: Arc<MemoryTierManager>,
    pub context_manager: Arc<ContextManager>,
    pub tool_selector: Arc<DynamicToolSelector>,
    pub episode_memory: Arc<EpisodeMemory>,
    pub model: String,
}

impl Brain {
    /// Build a new Brain from shared resources.
    pub async fn new(
        provider: Arc<dyn LLMProvider>,
        pool: sqlx::SqlitePool,
        registry: &ToolRegistry,
        model: String,
        context_config: ContextConfig,
    ) -> Result<Self> {
        let memory_tiering = Arc::new(MemoryTierManager::new(
            pool.clone(),
            Arc::clone(&provider),
            TieringConfig::default(),
        ));

        let context_manager = Arc::new(ContextManager::new(
            context_config,
            Some(Arc::clone(&provider)),
        ));

        let tool_selector = Arc::new(DynamicToolSelector::new(
            Arc::clone(&provider),
            10, // top_k
        ));
        tool_selector.build_cache(registry).await?;

        let episode_memory = Arc::new(EpisodeMemory::new(
            pool.clone(),
            Arc::clone(&provider),
            0.85,
        ));
        episode_memory.init_schema().await?;

        Ok(Self {
            provider,
            memory_tiering,
            context_manager,
            tool_selector,
            episode_memory,
            model,
        })
    }

    /// Select tools relevant to query (dynamic subset).
    pub async fn select_tools(
        &self,
        query: &str,
        registry: &ToolRegistry,
    ) -> Result<Vec<ToolSchema>> {
        self.tool_selector.select(query, registry).await
    }

    /// Build context with tiering + compression.
    pub async fn build_context(
        &self,
        session_id: &str,
        working_messages: &[ChatMessage],
        query: &str,
    ) -> Result<Vec<ChatMessage>> {
        // 1. Tiering: add recall + archival
        let tiered = self
            .memory_tiering
            .build_context(session_id, working_messages, query, 5)
            .await?;

        // 2. Compression: trim if too long
        let compressed = self.context_manager.build(&tiered).await?;

        Ok(compressed)
    }

    /// Maybe summarize conversation (call after each turn).
    pub async fn maybe_summarize(
        &self,
        session_id: &str,
        messages: &[ChatMessage],
    ) -> Result<Option<String>> {
        self.memory_tiering
            .maybe_summarize(session_id, messages)
            .await
    }

    /// Reflect on tool failure.
    pub async fn reflect(
        &self,
        tool: &str,
        input: &serde_json::Value,
        error: &str,
        history: &[ChatMessage],
    ) -> Result<ReflectionResult> {
        reflect_on_failure(&self.provider, &self.model, tool, input, error, history).await
    }

    /// Find similar failures (for agent context warning).
    pub async fn find_similar_failures(
        &self,
        tool: &str,
        input: &serde_json::Value,
    ) -> Result<Vec<SimilarFailure>> {
        self.episode_memory.find_similar_failures(tool, input).await
    }

    /// Record episode after tool execution.
    pub async fn record_episode(
        &self,
        tool: &str,
        input: &serde_json::Value,
        success: bool,
        error: Option<&str>,
        session_id: Option<&str>,
    ) -> Result<String> {
        self.episode_memory
            .record(tool, input, success, error, session_id)
            .await
    }

    /// Maybe generate plan for complex task.
    pub async fn maybe_plan(
        &self,
        user_message: &str,
        available_tools: &[&str],
    ) -> Result<Option<AgentPlan>> {
        if !should_plan(user_message) {
            return Ok(None);
        }
        let plan = generate_plan(&self.provider, &self.model, user_message, available_tools).await?;
        Ok(Some(plan))
    }

    /// Reset context cache (call when session changes).
    pub fn reset_context(&self) {
        self.context_manager.reset();
    }

    /// Run decay (call periodically or on idle).
    pub async fn run_decay(&self) -> Result<usize> {
        self.memory_tiering.run_decay().await
    }
}
