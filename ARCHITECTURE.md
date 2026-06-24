# NEXUS — Architecture Overview (Phase 1)

> Production-grade desktop AI Agent. Rust + Tauri v2 backend, React 18 + TypeScript + Vite frontend, SQLite persistence, multi-provider LLM abstraction, sandboxed tools, CDP browser automation, embedding-based long-term memory, and a cron-backed scheduler.

---

## 1. System Topology

```
┌────────────────────────────────────────────────────────────────────────────┐
│                              NEXUS Desktop App                              │
│                                                                              │
│  ┌──────────────────────────────┐    ┌──────────────────────────────────┐  │
│  │       Frontend (WebView)     │    │         Backend (Rust)            │  │
│  │  React 18 · TS · Tailwind    │    │  Tauri v2 · Tokio · SQLx          │  │
│  │  Zustand · Vite              │    │                                   │  │
│  │                              │    │  ┌────────────────────────────┐   │  │
│  │  ┌────────────────────────┐  │    │  │      Agent Engine          │   │  │
│  │  │ ChatView               │  │    │  │  ┌──────────────────────┐  │   │  │
│  │  │ ToolActivityPanel      │  │    │  │  │  Agent Loop (ReAct)  │  │   │  │
│  │  │ SessionSidebar         │  │    │  │  │  - reasoning         │  │   │  │
│  │  │ ApprovalDialog         │  │    │  │  │  - tool selection    │  │   │  │
│  │  │ MemoryPanel            │  │    │  │  │  - parallel exec     │  │   │  │
│  │  │ SchedulerPanel         │  │    │  │  │  - observe / iterate │  │   │  │
│  │  └────────────────────────┘  │    │  │  └──────────┬───────────┘  │   │  │
│  │                              │    │  │             │              │   │  │
│  │  IPC via tauri `invoke` +   │◄──►│  │  ┌──────────▼───────────┐  │   │  │
│  │  event listeners             │    │  │  │  ToolRegistry        │  │   │  │
│  │                              │    │  │  │  (auto-discovery)    │  │   │  │
│  └──────────────────────────────┘    │  │  └──────────┬───────────┘  │   │  │
│                                       │  │             │              │   │  │
│                                       │  │  ┌──────────▼───────────┐  │   │  │
│                                       │  │  │  LLMProvider trait   │  │   │  │
│                                       │  │  │  OpenAI · OpenRouter │  │   │  │
│                                       │  │  │  Anthropic · Ollama  │  │   │  │
│                                       │  │  └──────────────────────┘  │   │  │
│                                       │  └────────────────────────────┘   │  │
│                                       │                                    │  │
│                                       │  ┌────────────┐ ┌──────────────┐  │  │
│                                       │  │ Memory     │ │ Scheduler    │  │  │
│                                       │  │ (SQLx+emb)│ │ (cron, DB)   │  │  │
│                                       │  └────────────┘ └──────────────┘  │  │
│                                       │  ┌────────────┐ ┌──────────────┐  │  │
│                                       │  │ Browser    │ │ Security     │  │  │
│                                       │  │ (CDP, 1×)  │ │ (perm gate)  │  │  │
│                                       │  └────────────┘ └──────────────┘  │  │
│                                       │  ┌─────────────────────────────┐  │  │
│                                       │  │ SQLite (migrations, SQLx)   │  │  │
│                                       │  └─────────────────────────────┘  │  │
│                                       └──────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────────┘
                            External dependencies
            ┌───────────────┬──────────────┬───────────────┐
            ▼               ▼              ▼               ▼
       LLM APIs        DuckDuckGo     Brave Search    Local Chromium
   (OpenAI / Anthropic  (HTML scrape)   (REST API)    (CDP on :9222)
    / OpenRouter /
    Ollama)
```

---

## 2. Process & Runtime Model

- **Single Tauri process** hosts both the WebView (frontend) and the Rust core.
- The Rust core owns a single **Tokio multi-threaded runtime** (`Runtime::new()` wrapped in `tauri::State`), shared across all async modules.
- **Browser singleton**: a lazily-started `chromiumoxide::Browser` (CDP) wrapped in `tokio::sync::OnceCell` inside `AppState`. Only one Chromium instance is ever spawned per app lifetime.
- **Scheduler** runs its own background `tokio::task` driven by `tokio-cron-scheduler`. Jobs are restored from `tasks` table on startup.
- **Agent loop** runs inside a `tokio::task::spawn` per chat turn; cancellation is achieved via a `tokio_util::sync::CancellationToken` stored in `AppState::active_runs: Arc<DashMap<Uuid, CancellationToken>>`.
- **Tracing** is initialized once via `tracing_subscriber` with a layered format: console (debug) + rolling file appender (`logs/app.log`, 50 MB rotation, 7 days retained).

---

## 3. Module Breakdown

| Module | Crate path | Responsibility | Key types |
|---|---|---|---|
| `agent` | `src/agent/` | ReAct loop, tool selection, iteration/cancellation, streaming | `Agent`, `AgentLoop`, `AgentConfig`, `AgentEvent` |
| `tools` | `src/tools/` | `Tool` trait, registry, sandboxed implementations | `Tool`, `ToolRegistry`, `ToolCall`, `ToolResult` |
| `memory` | `src/memory/` | Short-term ring buffer + long-term SQLite store with embeddings | `MemoryStore`, `ShortTermMemory`, `MemoryEntry`, `EmbeddingClient` |
| `scheduler` | `src/scheduler/` | One-time + cron recurring jobs, persisted + restored | `SchedulerService`, `JobSpec`, `JobKind` |
| `browser` | `src/browser/` | Singleton CDP browser, lazy start, page operations | `BrowserManager`, `Page`, `BrowserAction` |
| `llm` | `src/llm/` | `LLMProvider` trait + 4 provider impls + streaming | `LLMProvider`, `ChatMessage`, `ChatRequest`, `ChatStream`, `ProviderConfig` |
| `commands` | `src/commands/` | Tauri `#[command]` IPC handlers (frontend-facing) | `chat_*`, `session_*`, `memory_*`, `scheduler_*`, `tool_*`, `browser_*` |
| `database` | `src/database/` | SQLx pool, migrations, repositories | `DbPool`, `migrations/`, `SessionRepo`, `MessageRepo`, etc. |
| `config` | `src/config/` | App config load/save, provider keys, workspace root | `AppConfig`, `ProviderConfig`, `ConfigStore` |
| `security` | `src/security/` | `PermissionLevel`, sandbox enforcement, approval flow | `PermissionLevel`, `Sandbox`, `ApprovalGate`, `CommandBlacklist` |

Each module is `pub mod` inside `lib.rs` / `main.rs` and exposes a typed API. No module reaches into another's internals — only its public surface. Cross-module dependencies flow strictly downward (agent → tools → security; agent → llm; agent → memory; never the reverse).

---

## 4. Agent Loop — Detailed Workflow

```
┌─────────────────────────────────────────────────────────────────────┐
│                       Agent::run(user_message)                      │
└─────────────────────────────────────────────────────────────────────┘
   │
   ▼
[1] Build system prompt (tools schema + workspace info + recent memory)
   │
   ▼
[2] Push user msg → short-term memory + DB (messages table)
   │
   ▼
[3] ┌──── ITERATION LOOP (max 10) ─────────────────────────────────┐
   │                                                               │
   │   3a. LLMProvider.chat_stream(messages, tools)                │
   │       - streams tokens → emits AgentEvent::Delta              │
   │       - emits AgentEvent::ToolCall(Vec<ToolCall>)             │
   │                                                               │
   │   3b. If no tool calls → emit FinalResponse, break.           │
   │                                                               │
   │   3c. For each ToolCall (parallel when safe):                 │
   │       ┌─────────────────────────────────────────┐             │
   │       │  i.   Look up tool in registry          │             │
   │       │  ii.  Resolve PermissionLevel           │             │
   │       │  iii. If !Safe → ApprovalGate.request   │             │
   │       │       (IPC event → frontend dialog)     │             │
   │       │       await user decision (or timeout)  │             │
   │       │  iv.  If approved → tool.execute(ctx)   │             │
   │       │  v.   Append ToolResult to messages     │             │
   │       │  vi.  Emit AgentEvent::ToolResult       │             │
   │       └─────────────────────────────────────────┘             │
   │                                                               │
   │   3d. Increment tool_call_count (max 50).                    │
   │                                                               │
   │   3e. Check CancellationToken → if cancelled, abort.          │
   │                                                               │
   └───────────────────────────────────────────────────────────────┘
   │
   ▼
[4] Persist final assistant message → DB
   │
   ▼
[5] Optionally write new long-term memories (agent-driven via tool)
```

**Concurrency rules:**
- Tool calls returned in a single LLM response that are all `Safe` and mutually independent (no shared file path) run in parallel via `futures::future::join_all`.
- `RequiresApproval` and `Dangerous` tools run sequentially to avoid races on the approval gate.
- Total per-turn tool invocations hard-capped at 50 (configurable in `AgentConfig`).

---

## 5. LLM Provider Abstraction

```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    fn id(&self) -> &'static str;
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, LlmError>;
    async fn chat_stream(
        &self,
        req: ChatRequest,
        tx: mpsc::Sender<ChatStreamChunk>,
    ) -> Result<(), LlmError>;
    async fn embed(&self, text: &str) -> Result<Vec<f32>, LlmError>;
    fn supports_tools(&self) -> bool;
}

pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub tools: Vec<ToolSchema>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

pub enum ChatStreamChunk {
    Delta(String),                       // token-by-token text
    ToolCall(ToolCall),                  // tool invocation request
    Usage { prompt_tokens: u32, completion_tokens: u32 },
    Done,
}
```

- `OpenAIProvider` — `reqwest` + `https://api.openai.com/v1`, SSE parsing for streaming, supports tool calling.
- `OpenRouterProvider` — same OpenAI-compatible schema, different base URL + headers, supports routing hints.
- `AnthropicProvider` — Claude Messages API (`/v1/messages`), tool use blocks, SSE event stream, `claude-3-5-sonnet` family.
- `OllamaProvider` — local `http://localhost:11434/api/chat`, supports streaming + tool calls for `llama3.1`, `qwen2.5`, `mistral` tool-enabled models.

Agent holds `provider: Arc<dyn LLMProvider>` resolved at startup from `AppConfig::active_provider`. Switching providers is a config change + restart of the agent runtime (no app restart needed).

---

## 6. Tool System

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn permission(&self) -> PermissionLevel;
    fn schema(&self) -> serde_json::Value;   // JSON Schema for LLM
    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value)
        -> Result<ToolResult, ToolError>;
}

pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Arc<dyn Tool>>>,
}

impl ToolRegistry {
    pub fn register<T: Tool + 'static>(&self, tool: T);
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>>;
    pub fn all_schemas(&self) -> Vec<ToolSchema>;
    pub fn auto_register_defaults(&self);  // called at startup
}
```

**Auto-registration**: every tool impl exposes a `register(registry: &ToolRegistry)` function. A `inventory`-style `submit!` macro is **not** used (we keep startup explicit for auditability); instead `tools::init()` is called once from `main.rs` and registers every tool in deterministic order.

**Tools to ship:**

| Category | Tools |
|---|---|
| File | `read_file`, `write_file`, `append_file`, `delete_file`, `move_file`, `copy_file`, `list_directory`, `search_files`, `create_directory` |
| Shell | `run_command` |
| Code | `run_python`, `run_javascript` |
| Browser | `browser_navigate`, `browser_click`, `browser_type`, `browser_wait`, `browser_extract_text`, `browser_screenshot` |
| Search | `web_search` (provider-selected at runtime) |
| Memory | `memory_save`, `memory_recall`, `memory_delete` |
| Scheduler | `schedule_one_time`, `schedule_recurring`, `list_scheduled`, `cancel_scheduled` |

---

## 7. Security Layer

### 7.1 Permission model

```rust
#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum PermissionLevel {
    Safe,             // auto-execute
    RequiresApproval, // ask user via IPC
    Dangerous,        // ask user + show extra warning
}
```

| Tool | Permission |
|---|---|
| `read_file`, `list_directory`, `search_files`, `browser_navigate`, `browser_wait`, `browser_extract_text`, `browser_screenshot`, `web_search`, `memory_recall` | Safe |
| `write_file`, `append_file`, `move_file`, `copy_file`, `create_directory`, `run_command`, `run_python`, `run_javascript`, `browser_click`, `browser_type`, `memory_save`, `schedule_*` | RequiresApproval |
| `delete_file`, `cancel_scheduled` | Dangerous |

### 7.2 Approval flow

```
Agent ──┐
        │  Tool requires approval
        ▼
ApprovalGate::request(call)  ───►  emit IPC event "approval:request"
                                                     │
                                                     ▼
                                          Frontend ApprovalDialog
                                                     │
                                            user clicks ✅ / ❌
                                                     │
                                          invoke("approval:respond", {id, decision})
                                                     │
        ┌────────────────────────────────────────────┘
        ▼
ApprovalGate resolves oneshot channel
        │
        ▼
Agent continues or aborts the tool call
```

- Each approval request has a 5-minute timeout. After timeout, the call is auto-rejected and the agent is informed via tool result.
- Approval decisions are **not** persisted across sessions (no "always allow") — this is a deliberate safety choice. Future enhancement: scoped "always allow for this session" toggle.

### 7.3 Filesystem sandbox

- Workspace root: `~/nexus_workspace` (created on first run).
- `Sandbox::resolve(path)` canonicalizes the path and rejects any path outside the workspace.
- **Hard-blocked absolute paths** (matched via `Path::starts_with` after canonicalization):
  - Unix: `/etc`, `/sys`, `/proc`, `/boot`, `/dev`, `/root`
  - Windows: `C:\Windows`, `C:\System32`, `C:\Program Files`
- Relative paths are resolved against the workspace root.
- Symlinks pointing outside the workspace are rejected.

### 7.4 Shell blacklist

`run_command` parses the command string and refuses execution if any of the following substrings/patterns appear (case-insensitive, after whitespace normalization):

- `rm -rf /`
- `rm -rf ~`
- `mkfs`
- `shutdown`
- `reboot`
- `:(){:|:&};:` (fork bomb) — plus a generic regex `:\(\)\s*\{.*\|.*&.*\}`
- `dd if=/dev/zero of=/dev/sd`
- `> /dev/sda`
- `chmod -R 777 /`
- `curl ... | sh` / `wget ... | sh` (remote script execution)

Additionally:
- Timeout: 60s default (configurable per call, max 600s).
- Output truncation: each of stdout/stderr capped at 256 KB; the rest is dropped with a `[truncated]` marker.
- Every command (approved/rejected/blacklisted/executed) is logged to `command_logs` table with full input, exit code, and durations.

---

## 8. Browser Automation

- Single `chromiumoxide::Browser` instance inside `AppState::browser: Arc<OnceCell<Arc<BrowserManager>>>`.
- **Lazy startup**: first call to any `browser_*` tool triggers `BrowserManager::start()` which spawns Chromium with `--headless=new --remote-debugging-port=9222`.
- Tab management: a single active page is reused; `browser_navigate` reuses the page if URL host changes, otherwise navigates in place.
- All browser tools have a 30s default timeout (configurable).
- Browser is **not** sandboxed by the workspace rule — it's an external process. Security is enforced via the permission gate (navigate = Safe, click/type = RequiresApproval).

---

## 9. Memory System

### 9.1 Short-term memory

- In-memory ring buffer of last 50 `(role, content)` pairs per active session.
- Lives in `AppState::short_term: DashMap<SessionId, ShortTermMemory>`.
- Cleared when a session is closed.

### 9.2 Long-term memory (SQLite + embeddings)

```sql
CREATE TABLE memories (
    id           TEXT PRIMARY KEY,
    content      TEXT NOT NULL,
    category     TEXT NOT NULL,        -- "fact" | "preference" | "task" | "note"
    tags         TEXT NOT NULL,        -- JSON array
    embedding    BLOB NOT NULL,        -- f32 little-endian, 1536-dim
    session_id   TEXT,                 -- nullable origin session
    created_at   INTEGER NOT NULL,
    last_used_at INTEGER NOT NULL,
    use_count    INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_memories_category ON memories(category);
CREATE INDEX idx_memories_last_used ON memories(last_used_at);
```

- Embeddings via OpenAI `text-embedding-3-small` (1536 dims). If the active provider is Ollama and no OpenAI key is configured, fall back to `nomic-embed-text` (768 dims) and pad/track dimension in a separate `embedding_dim` column (added in migration v2 to support both).
- **Cosine similarity search**: query embedding dot producted against every row's embedding, divided by product of norms. With ≤10k memories this brute-force scan completes in <5ms; we add a VPTree index (`sqlite-vss` extension) once the row count exceeds 5k (future enhancement, not in v1).
- **Deduplication**: before insert, run a similarity search at threshold 0.92. If a hit is found, update the existing row (merge tags, bump `use_count`, refresh `last_used_at`) instead of inserting a duplicate.

---

## 10. Scheduler

- Backed by `tokio-cron-scheduler` with a PostgreSQL-less `MemStore` for runtime scheduling, but every job is **also persisted** in the `tasks` table so it survives restart.
- On startup, `SchedulerService::restore_from_db()` reads all enabled jobs and re-adds them to the in-memory scheduler.
- Job kinds:
  - `OneTime { fire_at: DateTime<Utc>, message: String }` — fires once, then marks itself `completed` in DB.
  - `Recurring { cron: String, message: String }` — fires on every cron tick; the message is fed into the agent as a synthetic user message on the "scheduled" session.
- Natural language parsing ("Tomorrow 9AM", "Every day 9AM", "Every Monday") is done in Rust via `chrono` + a custom parser (`scheduler::nlp`), not delegated to the LLM (deterministic, testable).

---

## 11. Database Schema (overview — full SQL in Phase 4)

```
sessions
  id (TEXT PK) | title (TEXT) | created_at (INT) | updated_at (INT)
  provider (TEXT) | model (TEXT) | system_prompt (TEXT)

messages
  id (TEXT PK) | session_id (TEXT FK) | role (TEXT)
  content (TEXT) | tool_calls (TEXT)  -- JSON
  tool_results (TEXT)                  -- JSON
  created_at (INT)
  INDEX (session_id, created_at)

memories
  -- see §9.2

tasks
  id (TEXT PK) | kind (TEXT)           -- "one_time" | "recurring"
  payload (TEXT)                       -- JSON JobSpec
  cron (TEXT NULL) | fire_at (INT NULL)
  enabled (INT) | created_at (INT) | last_fired_at (INT NULL)

command_logs
  id (TEXT PK) | session_id (TEXT NULL)
  command (TEXT) | args (TEXT)         -- JSON
  status (TEXT)                        -- "approved"|"rejected"|"blacklisted"|"executed"|"timeout"
  exit_code (INT NULL) | stdout (TEXT NULL) | stderr (TEXT NULL)
  started_at (INT) | finished_at (INT NULL)
  INDEX (session_id, started_at)
```

- Migrations live in `src/database/migrations/` as numbered SQL files (`0001_init.sql`, `0002_embedding_dim.sql`, ...), applied via `sqlx::migrate!`.
- DB file: `~/nexus_workspace/nexus.db` (outside workspace sandbox so tools can't tamper with it).

---

## 12. IPC Contract (frontend ↔ backend)

Tauri commands (typed via `ts_rs` export to `frontend/src/bindings/`):

| Command | Direction | Purpose |
|---|---|---|
| `chat_send(session_id, text)` | FE→BE | Start an agent turn |
| `chat_cancel(run_id)` | FE→BE | Cancel active run |
| `session_create`, `session_list`, `session_delete`, `session_rename` | FE→BE | Session CRUD |
| `memory_save`, `memory_recall(query, k)`, `memory_list`, `memory_delete` | FE→BE | Memory ops |
| `scheduler_add`, `scheduler_list`, `scheduler_cancel` | FE→BE | Schedule ops |
| `tool_list`, `tool_invoke(name, input)` | FE→BE | Direct tool invocation (manual mode) |
| `config_get`, `config_set` | FE→BE | Read/write `AppConfig` |
| `approval_respond(request_id, decision)` | FE→BE | Resolve approval gate |

Tauri events (BE→FE, via `app.emit_to`):

| Event | Payload | Purpose |
|---|---|---|
| `agent:delta` | `{ run_id, session_id, text }` | Streaming token |
| `agent:tool_call` | `{ run_id, tool, input, call_id }` | Tool invocation started |
| `agent:tool_result` | `{ run_id, call_id, output, ok }` | Tool finished |
| `agent:done` | `{ run_id, session_id }` | Turn finished |
| `agent:error` | `{ run_id, message }` | Turn errored |
| `approval:request` | `{ request_id, tool, input, permission }` | Ask user |
| `scheduler:fired` | `{ task_id, message }` | Scheduled job triggered |

---

## 13. Error Handling Strategy

```rust
// Unified error type with thiserror
#[derive(thiserror::Error, Debug)]
pub enum NexusError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("llm provider error: {0}")]
    Llm(#[from] LlmError),

    #[error("tool error: {0}")]
    Tool(#[from] ToolError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("security: {0}")]
    Security(#[from] SecurityError),

    #[error("config: {0}")]
    Config(String),

    #[error("browser: {0}")]
    Browser(String),

    #[error("canceled")]
    Canceled,

    #[error("not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, NexusError>;
```

- **No `unwrap()` / `expect()` in production code** outside of tests and const initialization (verified via `clippy::unwrap_used` lint set to `deny` in CI).
- **No `panic!()`** in any non-test code path.
- IPC handlers convert `NexusError` → `serde_json::Value` with `{ code, message, details? }` and never leak internal paths or stack traces.
- All errors are logged with `tracing::error!` including the full error chain (`{error:?}`).

---

## 14. Observability

- **`tracing`** crate with spans for: `agent::run`, `agent::iteration`, `tool::execute`, `llm::chat_stream`, `db::query`, `scheduler::fire`.
- Log levels: `INFO` for app lifecycle, `DEBUG` for agent iterations, `TRACE` for tool I/O.
- Two layered subscribers:
  - `fmt` to stdout at `INFO` for dev visibility.
  - `tracing-appender::rolling::daily("logs", "app.log")` at `DEBUG` for persistent log; 7-day retention via `RollingFileAppender::Rotation::DAILY`.
- Structured fields on every span: `session_id`, `run_id`, `tool_name`, `provider`, `duration_ms`.

---

## 15. Testing Strategy (overview — full tests in Phase 7)

| Layer | Strategy | Coverage target |
|---|---|---|
| `security` | Unit tests for sandbox resolution, blacklist regex, permission matrix | ≥95% |
| `tools` (file/shell/code) | Integration tests against temp dirs, real subprocess for `run_command` | ≥85% |
| `llm` | Mock provider + recorded fixtures (VCR-style) for real API shape | ≥80% |
| `memory` | SQLite in-memory + fake embedding client, cosine similarity math tests | ≥90% |
| `scheduler` | Time-mocked scheduler, one-time + recurring + restore-from-DB | ≥85% |
| `agent` | Loop with mock LLM returning scripted tool calls, cancellation tests | ≥80% |
| `commands` | Tauri mock state, IPC handler contract tests | ≥70% |
| `browser` | Skipped in CI (requires Chromium); manual + smoke test only | n/a |

CI runs `cargo test --workspace --all-features` + `cargo clippy -- -D warnings` + `cargo tarpaulin --workspace --out Html` for coverage reporting.

---

## 16. Configuration

`~/nexus_workspace/config.toml`:

```toml
[agent]
max_iterations = 10
max_tool_calls = 50
default_provider = "openai"
default_model   = "gpt-4o-mini"

[llm.openai]
api_key = "..."        # can also be read from OPENAI_API_KEY env var
base_url = "https://api.openai.com/v1"

[llm.openrouter]
api_key = "..."
base_url = "https://openrouter.ai/api/v1"

[llm.anthropic]
api_key = "..."
base_url = "https://api.anthropic.com"

[llm.ollama]
base_url = "http://localhost:11434"
default_model = "llama3.1"

[memory]
embedding_provider = "openai"   # or "ollama"
recall_top_k = 5
dedup_threshold = 0.92

[security]
approval_timeout_secs = 300
shell_timeout_secs = 60
shell_max_output_kb = 256

[browser]
headless = true
port = 9222
```

Secrets are loaded in priority order: explicit config value → env var (`OPENAI_API_KEY`, etc.) → OS keyring via `keyring` crate (future enhancement).

---

## 17. Build & Run

- `cargo build --release` produces the Tauri binary.
- `npm run dev` (in `frontend/`) starts Vite in dev mode; Tauri dev mode hot-reloads both.
- `npm run tauri build` produces installers per platform (`.msi`, `.dmg`, `.AppImage`).
- First run creates `~/nexus_workspace/`, applies migrations, writes default config.

---

## 18. Phase 1 Deliverable Summary

This document is the **complete architectural blueprint** for NEXUS. Every subsequent phase maps directly to a section above:

- **Phase 2** → realizes the module layout described in §3.
- **Phase 3** → pins the dependency versions implied by §1, §2, §5–§10.
- **Phase 4** → implements `agent`, `llm`, `memory`, `scheduler`, `database`, `config`, `security`, `commands` per §4–§13.
- **Phase 5** → implements the tool table in §6 with the permission matrix in §7.1.
- **Phase 6** → builds the UI described in §12's IPC contract.
- **Phase 7** → executes the strategy in §15.

---

**End of Phase 1.** Awaiting your confirmation to proceed to **Phase 2 — Folder Structure**.
