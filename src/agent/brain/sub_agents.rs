//! Multi-agent handoff — sub-agents với roles riêng.
//!
//! Inspired by OpenAI Swarm pattern.
//!
//! Mỗi sub-agent có:
//! - `name`: identifier
//! - `instructions`: role-specific system prompt
//! - `tools`: subset của tool registry (chỉ những tool liên quan)
//!
//! Agent chính có thể "handoff" sang sub-agent khi task yêu cầu chuyên môn cụ thể.
//! Sub-agent chạy với context riêng, trả kết quả về cho agent chính.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::Result;
use crate::llm::provider::LLMProvider;
use crate::llm::types::{ChatMessage, ChatRequest, MessageRole};
use crate::tools::registry::ToolRegistry;
use crate::tools::schema::ToolSchema;

/// Sub-agent definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubAgent {
    /// Identifier (vd: "coder", "researcher", "planner").
    pub name: String,
    /// Role-specific system prompt.
    pub instructions: String,
    /// Tool names mà sub-agent có thể dùng.
    pub tools: Vec<String>,
    /// Display name cho UI.
    pub display_name: String,
}

impl SubAgent {
    /// Coder sub-agent — code execution + file ops.
    #[must_use]
    pub fn coder() -> Self {
        Self {
            name: "coder".into(),
            instructions: "You are a coding specialist. Your job is to write, run, and debug code. \
                Always test code before claiming it works. Use run_python or run_javascript to verify. \
                When fixing bugs, identify the root cause before patching."
                .into(),
            tools: vec![
                "read_file".into(),
                "write_file".into(),
                "append_file".into(),
                "list_directory".into(),
                "search_files".into(),
                "run_python".into(),
                "run_javascript".into(),
                "run_command".into(),
            ],
            display_name: "Coder".into(),
        }
    }

    /// Researcher sub-agent — web search + browser.
    #[must_use]
    pub fn researcher() -> Self {
        Self {
            name: "researcher".into(),
            instructions: "You are a research specialist. Your job is to find information online \
                and synthesize findings. Use web_search for facts, browser for interactive pages. \
                Cite sources (URLs) in your response."
                .into(),
            tools: vec![
                "web_search".into(),
                "browser_navigate".into(),
                "browser_click".into(),
                "browser_type".into(),
                "browser_wait".into(),
                "browser_extract_text".into(),
                "browser_screenshot".into(),
                "memory_save".into(),
                "memory_recall".into(),
            ],
            display_name: "Researcher".into(),
        }
    }

    /// Planner sub-agent — generates execution plans (no tools, pure reasoning).
    #[must_use]
    pub fn planner() -> Self {
        Self {
            name: "planner".into(),
            instructions: "You are a planning specialist. Given a user request, generate a step-by-step plan. \
                Do not execute tools — just produce a clear plan that other agents can follow."
                .into(),
            tools: vec!["list_scheduled".into(), "memory_recall".into()],
            display_name: "Planner".into(),
        }
    }

    /// File organizer sub-agent.
    #[must_use]
    pub fn file_organizer() -> Self {
        Self {
            name: "file_organizer".into(),
            instructions: "You are a file organization specialist. Your job is to organize files \
                in the workspace: create directory structures, move files, rename for consistency. \
                Always show what you're about to do before doing it."
                .into(),
            tools: vec![
                "list_directory".into(),
                "read_file".into(),
                "create_directory".into(),
                "move_file".into(),
                "copy_file".into(),
                "delete_file".into(),
                "search_files".into(),
            ],
            display_name: "File Organizer".into(),
        }
    }
}

/// Get all default sub-agents.
#[must_use]
pub fn default_sub_agents() -> Vec<SubAgent> {
    vec![
        SubAgent::coder(),
        SubAgent::researcher(),
        SubAgent::planner(),
        SubAgent::file_organizer(),
    ]
}

/// Detect if user message needs handoff to a sub-agent.
/// Returns `Some(SubAgent)` if handoff recommended.
#[must_use]
pub fn detect_handoff(user_message: &str) -> Option<SubAgent> {
    let lower = user_message.to_lowercase();

    // Coder triggers
    let coder_phrases = ["write code", "implement", "debug", "fix bug", "refactor", "python", "javascript", "function"];
    if coder_phrases.iter().any(|p| lower.contains(p)) {
        return Some(SubAgent::coder());
    }

    // Researcher triggers
    let researcher_phrases = ["search for", "find information", "research", "look up", "google", "browse to"];
    if researcher_phrases.iter().any(|p| lower.contains(p)) {
        return Some(SubAgent::researcher());
    }

    // Planner triggers
    let planner_phrases = ["plan", "strategy", "step by step", "how should i", "outline"];
    if planner_phrases.iter().any(|p| lower.contains(p)) {
        return Some(SubAgent::planner());
    }

    // File organizer triggers
    let file_phrases = ["organize files", "clean up", "tidy up", "sort files", "restructure"];
    if file_phrases.iter().any(|p| lower.contains(p)) {
        return Some(SubAgent::file_organizer());
    }

    None
}

/// Get tool schemas cho sub-agent (chỉ những tools trong sub-agent's tool list).
#[must_use]
pub fn sub_agent_schemas(sub_agent: &SubAgent, registry: &ToolRegistry) -> Vec<ToolSchema> {
    sub_agent
        .tools
        .iter()
        .filter_map(|name| registry.get(name))
        .map(|t| ToolSchema::new(t.name(), t.description(), t.schema()))
        .collect()
}

/// Build system prompt cho sub-agent — combines role instructions + tool list.
#[must_use]
pub fn sub_agent_system_prompt(sub_agent: &SubAgent, base_prompt: &str) -> String {
    let mut prompt = format!(
        "{base_prompt}\n\n\
         ## Your role: {} ({})\n\n\
         {}\n\n\
         ## Your tools (use only these):\n",
        sub_agent.display_name, sub_agent.name, sub_agent.instructions
    );
    for tool in &sub_agent.tools {
        prompt.push_str(&format!("- `{tool}`\n"));
    }
    prompt.push_str(
        "\nIf the task requires tools outside your scope, respond with `HANDOFF: <reason>` and \
         explain what kind of specialist is needed. The orchestrator will hand off to the right agent.",
    );
    prompt
}

/// Check if response is a handoff request.
#[must_use]
pub fn parse_handoff_request(response: &str) -> Option<String> {
    let lower = response.to_lowercase();
    if let Some(idx) = lower.find("handoff:") {
        let reason = response[idx + "handoff:".len()..].trim();
        if !reason.is_empty() {
            return Some(reason.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coder_sub_agent_has_coding_tools() {
        let coder = SubAgent::coder();
        assert!(coder.tools.contains(&"run_python".into()));
        assert!(coder.tools.contains(&"run_javascript".into()));
        assert!(coder.tools.contains(&"write_file".into()));
        assert!(!coder.tools.contains(&"web_search".into()));
    }

    #[test]
    fn researcher_sub_agent_has_search_tools() {
        let researcher = SubAgent::researcher();
        assert!(researcher.tools.contains(&"web_search".into()));
        assert!(researcher.tools.contains(&"browser_navigate".into()));
        assert!(!researcher.tools.contains(&"run_command".into()));
    }

    #[test]
    fn detect_handoff_coder() {
        assert!(detect_handoff("write a Python function to sort a list").is_some());
        assert!(detect_handoff("debug this error").is_some());
        let detected = detect_handoff("write code to compute fibonacci");
        assert_eq!(detected.unwrap().name, "coder");
    }

    #[test]
    fn detect_handoff_researcher() {
        let detected = detect_handoff("search for the latest Rust news");
        assert_eq!(detected.unwrap().name, "researcher");
    }

    #[test]
    fn detect_handoff_planner() {
        let detected = detect_handoff("plan how to refactor this codebase");
        assert_eq!(detected.unwrap().name, "planner");
    }

    #[test]
    fn detect_handoff_none_for_simple() {
        assert!(detect_handoff("hello").is_none());
        assert!(detect_handoff("read this file").is_none());
    }

    #[test]
    fn sub_agent_system_prompt_includes_role() {
        let coder = SubAgent::coder();
        let prompt = sub_agent_system_prompt(&coder, "BASE");
        assert!(prompt.contains("Coder"));
        assert!(prompt.contains("coding specialist"));
        assert!(prompt.contains("run_python"));
        assert!(prompt.contains("HANDOFF"));
    }

    #[test]
    fn parse_handoff_request_detects() {
        let response = "I need to write code but I'm a researcher. HANDOFF: needs coding specialist";
        let reason = parse_handoff_request(response);
        assert_eq!(reason.unwrap(), "needs coding specialist");
    }

    #[test]
    fn parse_handoff_request_returns_none_for_normal() {
        assert!(parse_handoff_request("Task done").is_none());
        assert!(parse_handoff_request("handoff:").is_none()); // empty reason
    }

    #[test]
    fn default_sub_agents_includes_all() {
        let agents = default_sub_agents();
        assert_eq!(agents.len(), 4);
        let names: Vec<_> = agents.iter().map(|a| a.name.as_str()).collect();
        assert!(names.contains(&"coder"));
        assert!(names.contains(&"researcher"));
        assert!(names.contains(&"planner"));
        assert!(names.contains(&"file_organizer"));
    }
}
