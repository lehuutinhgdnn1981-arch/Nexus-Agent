//! NEXUS — core library.
//!
//! Production-grade desktop AI Agent. Modules:
//! - [`agent`] — ReAct loop, streaming, cancellation
//! - [`llm`] — `LLMProvider` trait + 4 providers (OpenAI / OpenRouter / Anthropic / Ollama)
//! - [`tools`] — `Tool` trait + 24 tools (file / shell / code / browser / search / memory / scheduler)
//! - [`memory`] — short-term ring buffer + long-term SQLite + embeddings
//! - [`scheduler`] — cron + one-time jobs, NL parser
//! - [`browser`] — Chromium CDP singleton
//! - [`database`] — SQLx pool + 5 repositories + migrations
//! - [`config`] — TOML config + paths
//! - [`security`] — `PermissionLevel`, sandbox, approval gate, blacklist
//! - [`commands`] — Tauri `#[command]` IPC handlers

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(rust_2018_idioms)]

pub mod agent;
pub mod browser;
pub mod commands;
pub mod config;
pub mod database;
pub mod error;
pub mod llm;
pub mod memory;
pub mod observability;
pub mod scheduler;
pub mod security;
pub mod state;
pub mod tools;
pub mod utils;

/// Re-export prelude cho convenience.
pub mod prelude {
    pub use crate::agent::{Agent, AgentEvent, AgentRuntimeConfig};
    pub use crate::browser::BrowserManager;
    pub use crate::commands::IpcError;
    pub use crate::config::AppConfig;
    pub use crate::database::DbPool;
    pub use crate::error::{NexusError, Result};
    pub use crate::llm::{build_provider, ChatMessage, ChatRequest, LLMProvider};
    pub use crate::memory::MemoryStore;
    pub use crate::scheduler::SchedulerService;
    pub use crate::security::{PermissionLevel, Sandbox};
    pub use crate::state::AppState;
    pub use crate::tools::{Tool, ToolRegistry, ToolResult};
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
