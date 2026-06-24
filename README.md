# NEXUS

> Production-grade desktop AI Agent built with Rust + Tauri v2 + React 18.

![Status](https://img.shields.io/badge/status-v0.1.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)
![Rust](https://img.shields.io/badge/Rust-1.81+-orange)
![Tauri](https://img.shields.io/badge/Tauri-v2-yellow)

NEXUS is a fully-functional desktop AI agent capable of:
- 💬 Chatting with LLMs (OpenAI, OpenRouter, Anthropic, Ollama)
- 📁 File system operations (sandboxed to `~/<data_dir>/workspace/`)
- 🖥️ Shell command execution (blacklist + approval + timeout + log)
- 🐍 Code execution (Python, JavaScript)
- 🌐 Browser automation (Chromium CDP)
- 🔍 Web search (DuckDuckGo, Brave)
- 🧠 Long-term memory with embeddings (cosine similarity + dedup)
- ⏰ Task scheduling (one-time + cron recurring, NL parser)
- 📝 Session management with history

---

## Quick start

### Prerequisites
- Rust 1.81+ (`rustup`)
- Node.js 20+ (`nvm install 20`)
- Tauri v2 system dependencies:
  - **Linux**: `sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`
  - **macOS**: Xcode Command Line Tools
  - **Windows**: WebView2 runtime (preinstalled on Win11)
- Python 3 + Node.js (for code execution tools)
- Chromium (for browser automation tools)

### Install & run

```bash
# Clone
git clone <repo-url> nexus && cd nexus

# Install frontend deps
make frontend-install

# Dev mode (Tauri + Vite hot reload)
make dev
# → NEXUS window opens, frontend from Vite dev server

# Production build (creates installer)
cargo tauri build
# → src-tauri/target/release/bundle/{msi,dmg,AppImage}
```

### Configure LLM provider

Edit `~/<data_dir>/config.toml` (created on first run) or set env vars:

```bash
export OPENAI_API_KEY=sk-...
# or
export ANTHROPIC_API_KEY=sk-ant-...
# or for local
ollama serve  # then use provider="ollama"
```

---

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    NEXUS Desktop App                          │
│  ┌─────────────────────────┐  ┌───────────────────────────┐  │
│  │   Frontend (WebView)    │  │   Backend (Rust)          │  │
│  │   React 18 · TS ·       │  │   Tauri v2 · Tokio · SQLx │  │
│  │   Tailwind · Zustand    │  │                           │  │
│  │   Vite                  │  │   ┌─────────────────────┐ │  │
│  │                         │◄►│   │  Agent (ReAct loop) │ │  │
│  │  Sidebar · Chat ·       │  │   │  - max 10 iters     │ │  │
│  │  Tool Activity ·        │  │   │  - max 50 tool calls│ │  │
│  │  Approval · Memory ·    │  │   │  - streaming + canc │ │  │
│  │  Scheduler · Settings   │  │   └──────────┬──────────┘ │  │
│  └─────────────────────────┘  │              │            │  │
│                               │   ┌──────────▼──────────┐ │  │
│                               │   │  ToolRegistry (24)  │ │  │
│                               │   │  + PermissionGate   │ │  │
│                               │   └──────────┬──────────┘ │  │
│                               │              │            │  │
│                               │   ┌──────────▼──────────┐ │  │
│                               │   │  LLMProvider trait  │ │  │
│                               │   │  OpenAI · OpenRouter│ │  │
│                               │   │  Anthropic · Ollama │ │  │
│                               │   └─────────────────────┘ │  │
│                               │   Memory · Scheduler ·    │  │
│                               │   Browser (CDP) · SQLite  │  │
│                               └───────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

See [`ARCHITECTURE.md`](./ARCHITECTURE.md) for full design.

---

## Module map

| Module | Path | Purpose |
|---|---|---|
| `agent` | `src/agent/` | ReAct loop, streaming, cancellation |
| `llm` | `src/llm/` | `LLMProvider` trait + 4 provider impls + MockProvider for tests |
| `tools` | `src/tools/` | 24 tools across 7 categories (file/shell/code/browser/search/memory/scheduler) |
| `memory` | `src/memory/` | Short-term ring buffer + long-term SQLite + cosine similarity |
| `scheduler` | `src/scheduler/` | One-time + cron recurring jobs, NL parser, DB persistence |
| `browser` | `src/browser/` | Singleton Chromium CDP, lazy start, 6 page actions |
| `database` | `src/database/` | SQLx pool + 5 repositories + 3 migrations |
| `config` | `src/config/` | TOML config + atomic save + workspace paths |
| `security` | `src/security/` | PermissionLevel + Sandbox + ApprovalGate + Blacklist |
| `commands` | `src/commands/` | 21 Tauri `#[command]` IPC handlers |
| `observability` | `src/observability/` | Tracing + rolling daily file appender |
| `utils` | `src/utils/` | time/ids/truncate/json helpers |

---

## Custom LLM Providers

NEXUS supports any OpenAI-compatible endpoint via `[llm.custom.<id>]` in `config.toml`:

```toml
[llm.custom.together]
id = "together"
api_key = "..."                    # or set TOGETHER_API_KEY env var
base_url = "https://api.together.xyz/v1"
default_model = "meta-llama/Llama-3-70b-chat-hf"
supports_tools = true

[llm.custom.groq]
id = "groq"
api_key = "..."
base_url = "https://api.groq.com/openai/v1"
default_model = "llama-3.1-70b-versatile"

[llm.custom.vllm]
id = "vllm"
base_url = "http://localhost:8000/v1"
default_model = "meta-llama/Llama-3-8B"
supports_tools = false             # vLLM tool calling còn hạn chế
timeout_secs = 300

[llm.custom.lmstudio]
id = "lmstudio"
base_url = "http://localhost:1234/v1"
default_model = "local-model"
```

**Presets** (auto-detected from env vars on first run):
- `TOGETHER_API_KEY` → together provider
- `GROQ_API_KEY` → groq provider
- `MISTRAL_API_KEY` → mistral provider
- `DEEPSEEK_API_KEY` → deepseek provider

**Runtime management via IPC:**
- `custom_provider_add(config)` — add new provider at runtime
- `custom_provider_remove(id)` — remove provider
- `custom_provider_list()` — list all custom providers
- `provider_list_all()` — list all (built-in + custom)
- `custom_provider_preset(preset_id, api_key?)` — get preset config for known providers

**Compatible providers** (tested concept, all OpenAI-compatible):
- Together AI, Groq, Mistral AI, DeepSeek, Fireworks AI, Anyscale
- OpenRouter (sub-routes: `anthropic/claude-3.5-sonnet`, etc.)
- vLLM (local), LM Studio (local), LiteLLM proxy, Ollama OpenAI-compat mode
- Any self-hosted LLM server following OpenAI Chat Completions API

---

## Agent "brain" assessment

NEXUS uses a **ReAct (Reasoning + Action) loop** with max 10 iterations + max 50 tool calls per turn. See [`AGENT_BRAIN_UPGRADE.md`](./AGENT_BRAIN_UPGRADE.md) for full assessment and upgrade roadmap.

**Current strengths:**
- ReAct pattern, streaming + cancellation
- Permission-gated tool execution with approval flow
- Short-term + long-term memory with embeddings
- 24 tools across 7 categories
- Auto-discovery via `ToolRegistry`

**Recommended upgrades** (priority order):
1. Memory tiering (Working / Archival / Recall) — inspired by [letta/MemGPT](https://github.com/letta-ai/letta)
2. Plan-and-execute mode for complex tasks
3. Code agent pattern (LLM generates Python code) — inspired by [smolagents](https://github.com/huggingface/smolagents)
4. LangGraph-style state machine — inspired by [langgraph](https://github.com/langchain-ai/langgraph)
5. Multi-agent handoff — inspired by [openai/swarm](https://github.com/openai/swarm)
6. Integrate [rig](https://github.com/0xPlaygrounds/rig) Rust LLM framework

---

## Tools (24)

| Category | Tools | Permission |
|---|---|---|
| File | `read_file`, `write_file`, `append_file`, `delete_file`, `move_file`, `copy_file`, `list_directory`, `search_files`, `create_directory` | Safe → Dangerous |
| Shell | `run_command` | RequiresApproval (blacklist enforced) |
| Code | `run_python`, `run_javascript` | RequiresApproval |
| Browser | `browser_navigate`, `browser_click`, `browser_type`, `browser_wait`, `browser_extract_text`, `browser_screenshot` | Safe / RequiresApproval |
| Search | `web_search` | Safe |
| Memory | `memory_save`, `memory_recall`, `memory_delete` | Safe → Dangerous |
| Scheduler | `schedule_one_time`, `schedule_recurring`, `list_scheduled`, `cancel_scheduled` | Safe → Dangerous |

Run `make list-tools` to see all tools with permission badges.

---

## IPC contract

21 Tauri commands + 9 events. See [`bindings/types.ts`](./frontend/src/bindings/types.ts) for TypeScript types.

**Commands:**
- `chat_send`, `chat_cancel`
- `session_create`, `session_list`, `session_search`, `session_rename`, `session_delete`
- `memory_save`, `memory_recall`, `memory_list`, `memory_delete`
- `scheduler_add`, `scheduler_list`, `scheduler_cancel`
- `tool_list`, `tool_invoke`
- `browser_action`, `browser_shutdown`
- `config_get`, `config_set`
- `approval_respond`, `approval_pending`

**Events (backend → frontend):**
- `agent:turn_start`, `agent:delta`, `agent:tool_call`, `agent:tool_result`, `agent:iteration_end`, `agent:done`, `agent:error`, `agent:cancelled`
- `approval:request`
- `scheduler:fired`

---

## Testing

```bash
# All tests (unit + integration)
make test

# Unit only
make test-unit

# Integration only (needs `--features test-utils`)
make test-integration

# Browser smoke tests (needs Chromium running)
make test-browser

# Coverage report (HTML in coverage/)
make coverage
```

### Test counts (Phase 7)

| Layer | Tests |
|---|---|
| Unit tests (inline in modules) | ~80 |
| Integration tests (`tests/` dir) | ~50 |
| Agent loop tests (with MockProvider) | 7 |
| IPC contract tests | 17 |
| LLM streaming fixtures | 11 |
| Browser smoke tests (`--ignored`) | 4 |
| **Total** | **~170 tests** |

### Coverage

Target: ≥80% for core modules.

```bash
make coverage
open coverage/tarpaulin-report.html
```

---

## Security

### Permission levels

| Level | Behavior |
|---|---|
| `Safe` | Auto-execute |
| `RequiresApproval` | Frontend shows approval dialog, agent waits |
| `Dangerous` | Same as above + extra warning UI |

### Filesystem sandbox

- Workspace root: `~/<data_dir>/workspace/`
- Blocked paths: `/etc`, `/sys`, `/proc`, `/boot`, `/dev`, `/root`, `C:\Windows`, `C:\System32`, `C:\Program Files`
- Symlink escape detection
- Path normalization (`..` resolution)

### Shell blacklist

Refuses commands matching:
- `rm -rf /`, `rm -rf ~`, `rm -rf /*`
- `mkfs`, `shutdown`, `reboot`, `halt`
- `:(){ :|:& };:` (fork bomb)
- `curl ... | sh`, `wget ... | bash`
- `dd if=/dev/zero of=/dev/sd*`
- `> /dev/sda`
- `chmod -R 777 /`
- `format c:`, `diskpart`

All command executions (approved/rejected/blacklisted) are logged to `command_logs` table.

---

## Configuration

`~/<data_dir>/config.toml` (auto-created on first run):

```toml
[agent]
max_iterations = 10
max_tool_calls = 50
default_provider = "openai"
default_model = "gpt-4o-mini"

[llm.openai]
api_key = "..."        # or set OPENAI_API_KEY env var
base_url = "https://api.openai.com/v1"

[llm.anthropic]
api_key = "..."
base_url = "https://api.anthropic.com"

[llm.ollama]
base_url = "http://localhost:11434"
default_model = "llama3.1"

[memory]
embedding_provider = "openai"
embedding_model = "text-embedding-3-small"
embedding_dim = 1536
recall_top_k = 5
dedup_threshold = 0.92

[security]
approval_timeout_secs = 300
shell_timeout_secs = 60
shell_max_output_kb = 256

[browser]
headless = true
port = 9222

[search]
default = "duckduckgo"
brave_api_key = "..."
```

API keys can also be read from env vars: `OPENAI_API_KEY`, `OPENROUTER_API_KEY`, `ANTHROPIC_API_KEY`, `BRAVE_API_KEY`.

---

## CLI examples

```bash
# List all tools
make list-tools

# Run agent from CLI (debug without UI)
OPENAI_API_KEY=sk-... make run-cli ARGS="--session test --message 'list files in workspace'"

# Inspect memory + scheduler + command logs
make inspect
```

---

## Development

### Project structure

```
nexus/
├── src/                     # Lib crate `nexus` (core logic)
├── src-tauri/               # Binary crate `nexus-app` (Tauri entrypoint)
├── tests/                   # Integration tests
├── benches/                 # Criterion benchmarks
├── examples/                # CLI examples
├── frontend/                # React 18 + Vite + Tailwind
├── Cargo.toml               # Workspace root
├── tauri.conf.json          # Tauri config
├── capabilities/            # Tauri v2 ACL
├── .github/workflows/       # CI
├── Makefile                 # Dev shortcuts
└── ARCHITECTURE.md          # Full design doc
```

### Code quality

- `#![forbid(unsafe_code)]` in lib + binary
- `clippy::unwrap_used = "deny"`, `clippy::expect_used = "deny"`, `clippy::panic = "deny"`
- `#![warn(missing_docs)]`
- `rust_2018_idioms = "deny"`
- TypeScript strict mode + noUnusedLocals/noUnusedParameters

### Adding a new tool

1. Create `src/tools/<category>/<tool_name>.rs`:
   ```rust
   use async_trait::async_trait;
   use crate::error::{NexusError, Result};
   use crate::security::permission::PermissionLevel;
   use crate::tools::context::ToolContext;
   use crate::tools::tool::{Tool, ToolResult};

   pub struct MyTool;

   #[async_trait]
   impl Tool for MyTool {
       fn name(&self) -> &'static str { "my_tool" }
       fn description(&self) -> &'static str { "What it does" }
       fn permission(&self) -> PermissionLevel { PermissionLevel::Safe }
       fn schema(&self) -> serde_json::Value {
           serde_json::json!({ "type": "object", "properties": { /* ... */ } })
       }
       async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> Result<ToolResult> {
           // ...
           Ok(ToolResult::ok("", self.name(), "done"))
       }
   }
   ```

2. Register in `src/tools/<category>/mod.rs::register_all()`.

3. Add tests in `src/tools/<category>/tests.rs`.

4. Tool auto-appears in LLM tool list via `ToolRegistry::all_schemas()`.

### Adding a new LLM provider

1. Create `src/llm/<provider>.rs` implementing `LLMProvider` trait.
2. Add to `ProviderConfig` defaults in `src/config/provider_config.rs`.
3. Add match arm in `src/llm/factory.rs::build_provider()`.
4. Add config section in `src/config/app_config.rs::LlmConfig`.

---

## Roadmap

- [ ] Vector search index (sqlite-vss) for >5k memories
- [ ] OS keyring integration for API keys
- [ ] Multi-tab browser support
- [ ] Plugin system for user-defined tools
- [ ] Mobile companion app
- [ ] Multi-agent orchestration

---

## License

MIT © 2026 NEXUS Team
