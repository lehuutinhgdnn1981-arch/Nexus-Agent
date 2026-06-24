//! NEXUS — agent engine.
//!
//! V2 (upgraded): wires `brain` module vào `Agent::run` để enable:
//! - Memory tiering (Working/Archival/Recall)
//! - Context manager (token counting + compression)
//! - Plan-and-execute
//! - Reflection on failure
//! - Dynamic tool subset
//! - Episode memory
//! - Sub-agent handoff

pub mod agent;
pub mod brain;
pub mod config;
pub mod event;
pub mod loop_state;
pub mod prompt;

pub use agent::Agent;
pub use brain::{Brain, ContextConfig};
pub use config::AgentRuntimeConfig;
pub use event::AgentEvent;
pub use loop_state::LoopState;
