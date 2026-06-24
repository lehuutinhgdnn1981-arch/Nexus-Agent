//! NEXUS — error types
//!
//! Unified error hierarchy cho toàn bộ workspace. Mọi hàm public trả
//! `Result<T, NexusError>`. Không dùng `unwrap()` / `expect()` / `panic!()`
//! trong code production (được enforce bởi clippy `unwrap_used = deny`).

use thiserror::Error;

/// Lỗi từ LLM provider layer.
#[derive(Debug, Error)]
pub enum LlmError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("invalid API key for provider `{provider}`")]
    InvalidApiKey { provider: String },

    #[error("provider `{provider}` returned status {status}: {body}")]
    ProviderStatus {
        provider: String,
        status: u16,
        body: String,
    },

    #[error("stream error: {0}")]
    Stream(String),

    #[error("rate limited by provider `{provider}`")]
    RateLimited { provider: String },

    #[error("malformed response: {0}")]
    MalformedResponse(String),

    #[error("tool call not supported by provider `{0}`")]
    ToolNotSupported(String),

    #[error("embedding error: {0}")]
    Embedding(String),

    #[error("provider not configured: {0}")]
    NotConfigured(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Lỗi từ tool layer (sẽ re-export đầy đủ ở module tools).
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("tool `{0}` not found in registry")]
    NotFound(String),

    #[error("invalid input for tool `{tool}`: {reason}")]
    InvalidInput { tool: String, reason: String },

    #[error("tool execution failed: {0}")]
    Execution(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("sandbox violation: path `{0}` is outside workspace")]
    SandboxViolation(String),

    #[error("blacklisted command: {0}")]
    Blacklisted(String),

    #[error("timeout after {0:?}")]
    Timeout(std::time::Duration),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("operation cancelled by user")]
    Cancelled,
}

/// Lỗi từ security layer.
#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("sandbox violation: path `{0}` is outside workspace")]
    SandboxViolation(String),

    #[error("blocked system path: {0}")]
    BlockedSystemPath(String),

    #[error("symlink escape: `{0}` points outside workspace")]
    SymlinkEscape(String),

    #[error("blacklisted command: {0}")]
    Blacklisted(String),

    #[error("approval denied by user")]
    ApprovalDenied,

    #[error("approval timeout after {0:?}")]
    ApprovalTimeout(std::time::Duration),
}

/// Lỗi từ scheduler layer.
#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("invalid cron expression: {0}")]
    InvalidCron(String),

    #[error("invalid natural language time: {0}")]
    InvalidNaturalLanguage(String),

    #[error("job not found: {0}")]
    NotFound(String),

    #[error("persistence error: {0}")]
    Persistence(String),

    #[error("scheduler internal error: {0}")]
    Internal(String),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

/// Lỗi từ browser layer.
#[derive(Debug, Error)]
pub enum BrowserError {
    #[error("chromium not found in PATH. Install Chromium or set CHROME_PATH env var")]
    ChromiumNotFound,

    #[error("CDP connection failed: {0}")]
    CdpConnection(String),

    #[error("page not found: {0}")]
    PageNotFound(String),

    #[error("navigation failed: {0}")]
    Navigation(String),

    #[error("element not found: selector `{0}`")]
    ElementNotFound(String),

    #[error("timeout after {0:?}")]
    Timeout(std::time::Duration),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Lỗi từ config layer.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config file not found at {0}")]
    NotFound(String),

    #[error("config parse error: {0}")]
    Parse(String),

    #[error("config serialize error: {0}")]
    Serialize(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("toml deserialize error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("toml serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),
}

/// Unified error type cho toàn bộ workspace.
#[derive(Debug, Error)]
pub enum NexusError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("llm error: {0}")]
    Llm(#[from] LlmError),

    #[error("tool error: {0}")]
    Tool(#[from] ToolError),

    #[error("security error: {0}")]
    Security(#[from] SecurityError),

    #[error("scheduler error: {0}")]
    Scheduler(#[from] SchedulerError),

    #[error("browser error: {0}")]
    Browser(#[from] BrowserError),

    #[error("config error: {0}")]
    Config(#[from] ConfigError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde json error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("operation cancelled")]
    Cancelled,

    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("internal error: {0}")]
    Internal(String),
}

/// Result alias dùng trong toàn bộ workspace.
pub type Result<T, E = NexusError> = std::result::Result<T, E>;

/// Convert error thành JSON payload cho IPC (không leak stack trace / path nội bộ).
pub fn error_to_ipc_payload(err: &NexusError) -> serde_json::Value {
    let code = match err {
        NexusError::Database(_) => "database",
        NexusError::Llm(_) => "llm",
        NexusError::Tool(_) => "tool",
        NexusError::Security(_) => "security",
        NexusError::Scheduler(_) => "scheduler",
        NexusError::Browser(_) => "browser",
        NexusError::Config(_) => "config",
        NexusError::Io(_) => "io",
        NexusError::Serde(_) => "serde",
        NexusError::Http(_) => "http",
        NexusError::Cancelled => "cancelled",
        NexusError::NotFound(_) => "not_found",
        NexusError::InvalidArgument(_) => "invalid_argument",
        NexusError::Internal(_) => "internal",
    };
    serde_json::json!({
        "code": code,
        "message": err.to_string(),
    })
}
