//! LLM types — message, request, response, stream chunks.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::tools::schema::ToolSchema;

/// Role của message trong conversation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// Function call (OpenAI-style).
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ToolCallFunction {
    pub name: String,
    /// JSON string (lưu string để khớp với OpenAI API).
    pub arguments: String,
}

/// Một tool call từ assistant.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ToolCall {
    /// Unique ID cho tool call (LLM generate).
    pub id: String,
    /// Always "function" ở OpenAI/Anthropic/Ollama.
    #[serde(default = "default_tool_type")]
    pub r#type: String,
    pub function: ToolCallFunction,
}

fn default_tool_type() -> String {
    "function".into()
}

/// Một message trong conversation.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    /// Chỉ có khi role = Assistant và LLM request tool calls.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Chỉ có khi role = Tool — ID của tool call mà message này là kết quả.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Tên tool (chỉ khi role = Tool).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ChatMessage {
    #[must_use]
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    #[must_use]
    pub fn assistant_with_tools(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: Some(tool_calls),
            tool_call_id: None,
            name: None,
        }
    }

    #[must_use]
    pub fn tool_result(tool_call_id: impl Into<String>, name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
            name: Some(name.into()),
        }
    }
}

/// Request gửi tới LLM.
#[derive(Clone, Debug)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub tools: Vec<ToolSchema>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

impl ChatRequest {
    #[must_use]
    pub fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            model: model.into(),
            messages,
            tools: Vec::new(),
            temperature: None,
            max_tokens: None,
        }
    }

    #[must_use]
    pub fn with_tools(mut self, tools: Vec<ToolSchema>) -> Self {
        self.tools = tools;
        self
    }
}

/// Token usage stats.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, TS)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Non-streaming response.
#[derive(Clone, Debug)]
pub struct ChatResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
}

/// Chunk streaming từ LLM.
#[derive(Clone, Debug)]
pub enum ChatStreamChunk {
    /// Token text delta.
    Delta(String),
    /// LLM request tool call.
    ToolCall(ToolCall),
    /// Usage stats (cuối stream).
    Usage(Usage),
    /// Stream kết thúc.
    Done,
}
