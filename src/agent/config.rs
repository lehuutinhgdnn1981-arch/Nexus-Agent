//! Agent runtime configuration.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentRuntimeConfig {
    pub max_iterations: u32,
    pub max_tool_calls: u32,
    pub default_provider: String,
    pub default_model: String,
    pub system_prompt: Option<String>,
}

impl Default for AgentRuntimeConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            max_tool_calls: 50,
            default_provider: "openai".into(),
            default_model: "gpt-4o-mini".into(),
            system_prompt: None,
        }
    }
}
