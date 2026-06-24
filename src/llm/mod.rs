//! NEXUS — LLM provider abstraction.

pub mod anthropic;
pub mod custom;
pub mod error;
pub mod factory;
pub mod ollama;
pub mod openai;
pub mod openrouter;
pub mod provider;
pub mod streaming;
pub mod types;

#[cfg(any(test, feature = "test-utils"))]
pub mod mock;

pub use custom::{CustomProvider, CustomProviderConfig};
pub use error::LlmError;
pub use factory::build_provider;
pub use provider::LLMProvider;
pub use types::{ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, MessageRole, ToolCall, ToolCallFunction, Usage};

#[cfg(any(test, feature = "test-utils"))]
pub use mock::MockProvider;
