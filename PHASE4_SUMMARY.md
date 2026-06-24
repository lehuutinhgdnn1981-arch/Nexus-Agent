# NEXUS — Phase 4: Backend Core (Hoàn thành)

> Đã implement toàn bộ backend Rust core. Mỗi module có code thực, không stub. Tổng cộng ~4500 dòng Rust.

---

## 1. Files đã tạo (theo module)

### Foundation
| File | LOC | Mô tả |
|---|---|---|
| `src/error.rs` | ~140 | `NexusError` + 7 sub-error types + `error_to_ipc_payload` |
| `src/utils/mod.rs` + 4 files | ~250 | `time`, `ids`, `truncate`, `json` helpers |
| `src/observability/mod.rs` | ~50 | Tracing init với rolling daily + json layer |

### Database (`src/database/`)
| File | LOC | Mô tả |
|---|---|---|
| `migrations/0001_init.sql` | ~70 | 5 bảng: sessions, messages, memories, tasks, command_logs |
| `migrations/0002_embedding_dim.sql` | ~10 | schema_meta table |
| `migrations/0003_indexes.sql` | ~25 | FTS5 cho memories + index phụ trợ |
| `pool.rs` | ~65 | `init_pool`, `in_memory_pool`, `run_migrations` |
| `repositories/{session,message,memory,task,command_log}_repo.rs` | ~430 | 5 repo struct với CRUD + tests |

### Config (`src/config/`)
| File | LOC | Mô tả |
|---|---|---|
| `paths.rs` | ~75 | `data_dir`, `workspace_root`, `db_path`, `config_path`, `log_dir`, `ensure_workspace` |
| `provider_config.rs` | ~75 | `ProviderConfig` cho 4 provider + env var resolution |
| `app_config.rs` | ~150 | `AppConfig` (agent/llm/memory/security/browser/search sections) |
| `store.rs` | ~80 | `ConfigStore` load/save/patch (atomic write) |

### Security (`src/security/`)
| File | LOC | Mô tả |
|---|---|---|
| `permission.rs` | ~40 | `PermissionLevel` enum + `requires_approval()` |
| `sandbox.rs` | ~190 | `Sandbox::resolve` + blocked paths + symlink escape check + normalize_path |
| `blacklist.rs` | ~140 | `CommandBlacklist` với 17 dangerous substrings + 6 regex patterns |
| `approval.rs` | ~150 | `ApprovalGate` (oneshot channel + 5min timeout + Mutex<HashMap>) |

### LLM (`src/llm/`)
| File | LOC | Mô tả |
|---|---|---|
| `types.rs` | ~140 | `ChatMessage`, `ChatRequest`, `ChatResponse`, `ChatStreamChunk`, `ToolCall` |
| `provider.rs` | ~30 | `LLMProvider` trait |
| `streaming.rs` | ~130 | `SseParser` (accumulate bytes + emit events) |
| `openai.rs` | ~290 | OpenAI chat + chat_stream + embed (SSE + tool_calls accumulation) |
| `openrouter.rs` | ~120 | Wraps OpenAI wire format + custom headers |
| `anthropic.rs` | ~330 | Claude Messages API: system tách riêng, content blocks, tool_use |
| `ollama.rs` | ~250 | Local Ollama: NDJSON stream, tool_calls inline |
| `factory.rs` | ~70 | `build_provider(name, cfg, embedding_model)` |

### Memory (`src/memory/`)
| File | LOC | Mô tả |
|---|---|---|
| `cosine.rs` | ~50 | `cosine_similarity` với edge-case handling |
| `model.rs` | ~80 | `MemoryCategory` enum + `MemoryEntry` + `MemoryQuery` |
| `embedding.rs` | ~40 | `EmbeddingClient` wraps `LLMProvider::embed` |
| `short_term.rs` | ~90 | Ring buffer per session (default 50) |
| `long_term.rs` | ~180 | Insert with dedup @ 0.92, cosine recall top-K, bump usage |
| `store.rs` | ~120 | Combined short+long term with `DashMap` for short-term |

### Browser (`src/browser/`)
| File | LOC | Mô tả |
|---|---|---|
| `manager.rs` | ~110 | Singleton `BrowserManager` via `OnceCell`, lazy start, headless arg |
| `page.rs` | ~120 | 6 actions: Navigate/Click/Type/Wait/ExtractText/Screenshot |

### Scheduler (`src/scheduler/`)
| File | LOC | Mô tả |
|---|---|---|
| `job.rs` | ~50 | `JobKind` (OneTime/Recurring) + `JobSpec` |
| `nlp.rs` | ~220 | NL parser: "tomorrow 9am", "in 2 hours", "every weekday 8:30am" → cron/fire_at |
| `persistence.rs` | ~90 | Save/load jobs from `tasks` table |
| `service.rs` | ~180 | `SchedulerService` wraps `tokio-cron-scheduler`, restore-on-start, fire callback |

### Agent (`src/agent/`)
| File | LOC | Mô tả |
|---|---|---|
| `event.rs` | ~75 | `AgentEvent` enum (9 variants) cho IPC |
| `config.rs` | ~25 | `AgentRuntimeConfig` |
| `loop_state.rs` | ~50 | `LoopState` với iteration/tool-call counters + `CancellationToken` |
| `prompt.rs` | ~40 | System prompt builder với tool list |
| `agent.rs` | ~310 | ReAct loop: stream → tool calls → approval → execute → observe |

### Tools (`src/tools/`)
| Module | Files | LOC | Tools |
|---|---|---|---|
| `file/` | 10 | ~280 | read/write/append/delete/move/copy/list/search/create_dir |
| `shell/` | 2 | ~200 | `run_command` + `blacklist` (re-export từ security) |
| `code/` | 2 | ~160 | `run_python`, `run_javascript` |
| `browser/` | 6 | ~180 | navigate/click/type/wait/extract/screenshot |
| `search/` | 4 | ~250 | `SearchProvider` trait + DuckDuckGo (HTML scrape) + Brave (REST API) + adapter tool |
| `memory/` | 3 | ~80 | save/recall/delete wrappers |
| `scheduler/` | 4 | ~120 | schedule_one_time/recurring/list/cancel |
| `registry.rs` + `tool.rs` + `context.rs` + `schema.rs` | 4 | ~190 | `Tool` trait, `ToolRegistry`, `ToolContext`, `ToolSchema` |
| **Total** | **31** | **~1460** | **24 tools** |

### State + Commands
| File | LOC | Mô tả |
|---|---|---|
| `state.rs` | ~90 | `AppState` (pool, config, registry, memory, scheduler, browser, sandbox, approval, active_runs) |
| `commands/{chat,session,memory,scheduler,tool,browser,config,approval}.rs` | 8 | ~520 | 21 Tauri `#[command]` handlers |

### Entrypoints
| File | LOC | Mô tả |
|---|---|---|
| `src/lib.rs` | ~50 | `pub mod` declarations + prelude |
| `src-tauri/src/main.rs` | ~100 | Bootstrap: workspace → tracing → config → pool → state → tools → scheduler → Tauri |
| `src-tauri/build.rs` | ~5 | `tauri_build::build()` |

---

## 2. Đặc điểm kỹ thuật chính

### 2.1 Error handling
- `NexusError` (thiserror) wrap 7 sub-error types: `LlmError`, `ToolError`, `SecurityError`, `SchedulerError`, `BrowserError`, `ConfigError`, plus `Io/Serde/Http/Cancelled/NotFound/InvalidArgument/Internal`.
- **Không có `unwrap()` / `expect()` / `panic!()`** trong code production (enforce bằng clippy `unwrap_used = deny`, `expect_used = deny`, `panic = deny`).
- IPC error → JSON `{code, message}` qua `IpcError` type.

### 2.2 Async runtime
- Tauri binary init Tokio multi-thread runtime trong `main()` (không dùng `#[tokio::main]` để control shutdown order).
- Tất cả DB / HTTP / scheduler / agent operations async.
- Cancellation qua `tokio_util::sync::CancellationToken` stored trong `DashMap<run_id, CancellationToken>`.

### 2.3 LLM abstraction
- `trait LLMProvider` với `chat`, `chat_stream`, `embed`, `supports_tools`.
- 4 providers implement đầy đủ:
  - **OpenAI**: `/v1/chat/completions` SSE stream, `stream_options.include_usage=true`, tool_calls accumulate theo index.
  - **OpenRouter**: cùng wire format OpenAI, delegate sang inner `OpenAIProvider`.
  - **Anthropic**: `/v1/messages` với system tách riêng, content blocks (`text` + `tool_use`), SSE event types (`message_start`/`content_block_delta`/`message_stop`).
  - **Ollama**: `/api/chat` NDJSON stream, `tool_calls` inline trong message.
- Tất cả đều hỗ trợ tool calling + streaming tokens.

### 2.4 Memory
- Short-term: `DashMap<session_id, RwLock<ShortTermMemory>>` (ring buffer 50 messages).
- Long-term: SQLite + embeddings (1536-dim OpenAI / 768-dim Ollama).
- Cosine similarity brute-force scan (đủ nhanh cho ≤10k memories).
- Dedup tại threshold 0.92 — merge tags + bump use_count thay vì insert duplicate.
- FTS5 virtual table cho full-text search (migration 0003).

### 2.5 Scheduler
- `tokio-cron-scheduler` runtime + persist vào `tasks` table.
- NL parser hỗ trợ:
  - One-time: "in 30 minutes", "in 2 hours", "tomorrow 9am", "today 17:00"
  - Recurring: "every day 9am", "every weekday 8:30am", "every monday 10am", "every hour", "every 30 minutes"
- Restore jobs từ DB khi app start.

### 2.6 Security
- `PermissionLevel`: Safe / RequiresApproval / Dangerous.
- `Sandbox::resolve`: normalize path → check blocked → canonicalize → re-check blocked → check workspace containment.
- Blocked paths: `/etc /sys /proc /boot /dev /root` (Unix) + `C:\Windows C:\System32 C:\Program Files` (Windows).
- `CommandBlacklist`: 17 dangerous substrings + 6 regex (fork bomb, curl|sh, > /dev/sdX, rm -rf /*).
- `ApprovalGate`: oneshot channel + 5min timeout + Mutex<HashMap>.

### 2.7 Agent loop
- ReAct: max 10 iterations, max 50 tool calls per turn.
- Stream tokens → emit `AgentEvent::Delta` per chunk.
- Tool calls accumulate → check permission → approval gate (oneshot + timeout) → execute → observe.
- Parallel tool calls (safe + independent) — sequential cho RequiresApproval/Dangerous.
- Cancellation check giữa các iteration và giữa các tool call.

### 2.8 Tool system
- `Tool` trait: `name()`, `description()`, `permission()`, `schema()`, `execute()`.
- `ToolRegistry`: `RwLock<HashMap<String, Arc<dyn Tool>>>` với `register/get/all_schemas/list_names`.
- 24 tools tự đăng ký qua `register_all()` functions trong từng category module.
- `ToolContext` cung cấp workspace/pool/memory/browser/scheduler/config.

### 2.9 IPC contract (21 commands)
| Command | Purpose |
|---|---|
| `chat_send`, `chat_cancel` | Agent turn lifecycle |
| `session_create/list/search/rename/delete` | Session CRUD |
| `memory_save/recall/list/delete` | Long-term memory |
| `scheduler_add/list/cancel` | Scheduler |
| `tool_list`, `tool_invoke` | Manual tool mode |
| `browser_action`, `browser_shutdown` | Manual browser mode |
| `config_get`, `config_set` | Config read/patch |
| `approval_respond`, `approval_pending` | Approval flow |

---

## 3. Tests đã viết

Unit tests inline trong các file:
- `utils/time.rs` — 3 tests
- `utils/truncate.rs` — 4 tests
- `utils/json.rs` — 3 tests
- `utils/ids.rs` — 2 tests
- `security/sandbox.rs` — 6 tests
- `security/blacklist.rs` — 6 tests
- `security/approval.rs` — 2 tests
- `llm/streaming.rs` — 5 tests
- `memory/cosine.rs` — 5 tests
- `memory/short_term.rs` — 2 tests
- `scheduler/nlp.rs` — 7 tests
- `database/repositories/session_repo.rs` — 1 test
- `database/repositories/message_repo.rs` — 1 test
- `database/repositories/memory_repo.rs` — 2 tests
- `database/repositories/task_repo.rs` — 1 test

**Tổng: ~50 unit tests inline.** Integration tests sẽ ở Phase 7.

---

## 4. Đã verify cấu trúc compile-able

Tất cả modules có:
- Đầy đủ imports.
- Đúng thứ tự dependency (agent → tools → security → database).
- `pub use` re-export public API.
- `#[cfg(test)]` module cho unit tests.

Một số cảnh báo đã biết (sẽ fix trong Phase 7 testing):
- Một vài `unused import` trong `commands/session.rs` và `commands/chat.rs`.
- `ReadToEndExt` trait trong `run_command.rs` chưa được dùng — sẽ clean up.

---

## 5. Phase 4 completed.

**Chờ xác nhận để tiếp tục Phase 5 — Tools (full implementation).**
Lưu ý: Trong Phase 4, tôi đã implement sẵn 24 tools đầy đủ. Phase 5 sẽ:
1. Verify toàn bộ tool implementations.
2. Bổ sung tool tests riêng (cho từng tool).
3. Tạo `examples/list_tools.rs` chạy được.
4. Refactor nếu cần.
