# NEXUS — Cấu trúc thư mục (Phase 2)

> Cấu trúc dự án NEXUS theo nguyên tắc: tách bạch backend Rust (Tauri v2) và frontend React 18 (Vite), mỗi module Rust độc lập với public API rõ ràng, migrations SQL nằm cùng code, tests tách unit/integration.

---

## 1. Layout tổng thể

```
nexus/
├── ARCHITECTURE.md                  # Phase 1 — blueprint kiến trúc
├── FOLDER_STRUCTURE.md              # Phase 2 — file này
├── README.md                        # Hướng dẫn build/run (sinh ở Phase cuối)
├── .gitignore
├── .editorconfig
├── rust-toolchain.toml              # pin Rust nightly/stable
├── .cargo/
│   └── config.toml                  # cargo target dir, registry mirror tuỳ chọn
│
├── Cargo.toml                       # workspace root
├── Cargo.lock
├── tauri.conf.json                  # Tauri v2 config (app identity, windows, bundle)
├── build.rs                         # Tauri build hook (frontend dist + bundle resources)
├── icons/                           # icon app (.ico/.icns/.png) — 5 kích thước chuẩn Tauri
│   ├── 32x32.png
│   ├── 128x128.png
│   ├── 128x128@2x.png
│   ├── icon.icns
│   └── icon.ico
├── capabilities/
│   └── default.json                 # Tauri v2 capability ACL (filesystem, shell, http...)
│
├── src-tauri/                       # THAY ĐỔI: dùng thư mục chuẩn Tauri v2
│   ├── Cargo.toml                   # manifest binary chính (tauri app)
│   ├── tauri.conf.json              # symlink → ../tauri.conf.json (hoặc copy)
│   ├── build.rs
│   └── src/
│       └── main.rs                  # entrypoint: init runtime + state + IPC + launch
│
├── src/                             # CRATE THƯ VIỆN NEXUS (logic dùng chung)
│   ├── lib.rs                       # khai báo pub mod + re-export API công khai
│   ├── prelude.rs                   # import tiện ích cho người dùng crate
│   │
│   ├── error.rs                     # NexusError + Result<T> + SecurityError + ToolError + LlmError
│   │
│   ├── state.rs                     # AppState: pool, registry, scheduler, browser, approval gate
│   │
│   ├── agent/                       # === AGENT ENGINE (§4 ARCHITECTURE) ===
│   │   ├── mod.rs                   # pub use + Agent::run
│   │   ├── agent.rs                 # struct Agent, run(), iterate()
│   │   ├── loop_state.rs            # AgentLoopState: iteration count, tool count, cancelled
│   │   ├── event.rs                 # AgentEvent enum (Delta, ToolCall, ToolResult, Done, Error)
│   │   ├── prompt.rs                # system prompt builder + tool schema formatter
│   │   ├── config.rs                # AgentConfig (max_iterations, max_tool_calls, provider)
│   │   └── tests.rs                 # unit tests cho loop logic (mock provider)
│   │
│   ├── llm/                         # === LLM PROVIDER ABSTRACTION (§5) ===
│   │   ├── mod.rs
│   │   ├── provider.rs              # trait LLMProvider
│   │   ├── types.rs                 # ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk
│   │   ├── error.rs                 # LlmError
│   │   ├── openai.rs                # OpenAIProvider
│   │   ├── openrouter.rs            # OpenRouterProvider
│   │   ├── anthropic.rs             # AnthropicProvider
│   │   ├── ollama.rs                # OllamaProvider
│   │   ├── streaming.rs             # SSE parser dùng cho OpenAI/Anthropic
│   │   ├── factory.rs               # build_provider(config) -> Arc<dyn LLMProvider>
│   │   └── tests.rs
│   │
│   ├── tools/                       # === TOOL SYSTEM (§6 ARCHITECTURE + Phase 5) ===
│   │   ├── mod.rs                   # pub use + init()
│   │   ├── tool.rs                  # trait Tool, ToolContext, ToolResult, ToolError
│   │   ├── registry.rs              # ToolRegistry (register/get/all_schemas/auto_register)
│   │   ├── schema.rs                # ToolSchema (JSON Schema cho LLM)
│   │   ├── context.rs               # ToolContext: workspace, db, http, browser, scheduler
│   │   ├── file/                    # 9 tool filesystem
│   │   │   ├── mod.rs               # register_all()
│   │   │   ├── read_file.rs
│   │   │   ├── write_file.rs
│   │   │   ├── append_file.rs
│   │   │   ├── delete_file.rs
│   │   │   ├── move_file.rs
│   │   │   ├── copy_file.rs
│   │   │   ├── list_directory.rs
│   │   │   ├── search_files.rs
│   │   │   └── create_directory.rs
│   │   ├── shell/
│   │   │   ├── mod.rs
│   │   │   ├── run_command.rs
│   │   │   └── blacklist.rs         # COMMAND_BLACKLIST + regex fork bomb
│   │   ├── code/
│   │   │   ├── mod.rs
│   │   │   ├── run_python.rs
│   │   │   └── run_javascript.rs
│   │   ├── browser/
│   │   │   ├── mod.rs
│   │   │   ├── navigate.rs
│   │   │   ├── click.rs
│   │   │   ├── type_text.rs
│   │   │   ├── wait.rs
│   │   │   ├── extract_text.rs
│   │   │   └── screenshot.rs
│   │   ├── search/
│   │   │   ├── mod.rs
│   │   │   ├── provider.rs          # trait SearchProvider
│   │   │   ├── duckduckgo.rs
│   │   │   ├── brave.rs
│   │   │   └── web_search_tool.rs   # adapter Tool::execute
│   │   ├── memory/                  # tool wrapper quanh memory store
│   │   │   ├── mod.rs
│   │   │   ├── save.rs
│   │   │   ├── recall.rs
│   │   │   └── delete.rs
│   │   ├── scheduler/               # tool wrapper quanh scheduler service
│   │   │   ├── mod.rs
│   │   │   ├── schedule_one_time.rs
│   │   │   ├── schedule_recurring.rs
│   │   │   ├── list_scheduled.rs
│   │   │   └── cancel_scheduled.rs
│   │   └── tests.rs                 # test registry, schema gen, sandbox
│   │
│   ├── memory/                      # === MEMORY SYSTEM (§9 ARCHITECTURE) ===
│   │   ├── mod.rs
│   │   ├── store.rs                 # MemoryStore: short-term ring + long-term SQLite
│   │   ├── short_term.rs            # ShortTermMemory ring buffer 50 entries
│   │   ├── long_term.rs             # LongTermMemory: insert, cosine search, dedup
│   │   ├── embedding.rs             # EmbeddingClient: OpenAI / Ollama
│   │   ├── cosine.rs                # hàm cosine_similarity(a, b)
│   │   ├── model.rs                 # MemoryEntry, MemoryCategory, MemoryQuery
│   │   └── tests.rs
│   │
│   ├── scheduler/                   # === SCHEDULER (§10 ARCHITECTURE) ===
│   │   ├── mod.rs
│   │   ├── service.rs               # SchedulerService: start/restore/add/cancel
│   │   ├── job.rs                   # JobSpec, JobKind, JobId
│   │   ├── nlp.rs                   # parse_natural_language → cron/fire_at
│   │   ├── persistence.rs           # save/load tasks table
│   │   └── tests.rs
│   │
│   ├── browser/                     # === BROWSER AUTOMATION (§8) ===
│   │   ├── mod.rs
│   │   ├── manager.rs               # BrowserManager: singleton, lazy start, shutdown
│   │   ├── page.rs                  # Page abstraction (navigate/click/type/wait/extract/screenshot)
│   │   ├── error.rs                 # BrowserError
│   │   └── tests.rs                 # smoke test (skip trong CI)
│   │
│   ├── commands/                    # === IPC HANDLERS (§12) ===
│   │   ├── mod.rs                   # đăng ký tất cả #[tauri::command] vào invoke_handler
│   │   ├── chat.rs                  # chat_send, chat_cancel
│   │   ├── session.rs               # session_create/list/delete/rename
│   │   ├── memory.rs                # memory_save/recall/list/delete
│   │   ├── scheduler.rs             # scheduler_add/list/cancel
│   │   ├── tool.rs                  # tool_list, tool_invoke
│   │   ├── browser.rs               # browser_* (manual mode)
│   │   ├── config.rs                # config_get, config_set
│   │   ├── approval.rs              # approval_respond
│   │   └── tests.rs                 # contract test với mock AppState
│   │
│   ├── database/                    # === DATABASE LAYER (§11) ===
│   │   ├── mod.rs
│   │   ├── pool.rs                  # DbPool = SqlitePool, init + migrate
│   │   ├── migrations/              # SQLx embed
│   │   │   ├── 0001_init.sql        # sessions, messages, memories, tasks, command_logs
│   │   │   ├── 0002_embedding_dim.sql
│   │   │   └── 0003_indexes.sql
│   │   ├── repositories/
│   │   │   ├── mod.rs
│   │   │   ├── session_repo.rs
│   │   │   ├── message_repo.rs
│   │   │   ├── memory_repo.rs
│   │   │   ├── task_repo.rs
│   │   │   └── command_log_repo.rs
│   │   └── tests.rs
│   │
│   ├── config/                      # === CONFIG (§16) ===
│   │   ├── mod.rs
│   │   ├── app_config.rs            # AppConfig (toml de/serialize)
│   │   ├── provider_config.rs       # ProviderConfig cho 4 provider
│   │   ├── store.rs                 # ConfigStore: load/save/patch
│   │   ├── paths.rs                 # workspace_root(), db_path(), log_dir()
│   │   └── tests.rs
│   │
│   ├── security/                    # === SECURITY (§7) ===
│   │   ├── mod.rs
│   │   ├── permission.rs            # PermissionLevel enum
│   │   ├── sandbox.rs               # Sandbox::resolve, blocked paths
│   │   ├── approval.rs              # ApprovalGate, ApprovalRequest, ApprovalDecision
│   │   ├── blacklist.rs             # shell command blacklist + regex
│   │   └── tests.rs                 # ≥95% coverage
│   │
│   ├── observability/               # === TRACING / LOGS (§14) ===
│   │   ├── mod.rs
│   │   └── init.rs                  # tracing_subscriber + rolling appender
│   │
│   └── utils/                       # helpers dùng chung
│       ├── mod.rs
│       ├── time.rs                  # now_ts() -> i64, parse_iso(...)
│       ├── ids.rs                   # new_uuid() -> String
│       ├── truncate.rs              # truncate_output(stdout, stderr, max_kb)
│       └── json.rs                  # JSON helpers (merge, get_path)
│
├── tests/                           # INTEGRATION TESTS (cargo test --test)
│   ├── common/
│   │   └── mod.rs                   # fixtures: temp workspace, mock provider, in-memory db
│   ├── agent_loop_integration.rs    # end-to-end agent với mock LLM
│   ├── tools_file_integration.rs    # file tools trên temp dir
│   ├── tools_shell_integration.rs   # run_command + blacklist
│   ├── tools_code_integration.rs    # python/node subprocess
│   ├── memory_integration.rs        # store + recall + dedup
│   ├── scheduler_integration.rs     # one-time + recurring + restore
│   ├── security_sandbox_integration.rs
│   └── ipc_contract_tests.rs        # Tauri command shape
│
├── benches/                         # criterion benchmarks
│   ├── cosine_search.rs
│   └── agent_loop.rs
│
├── examples/                        # ví dụ standalone để debug từng module
│   ├── run_agent_cli.rs             # chạy agent từ CLI (không UI)
│   ├── list_tools.rs
│   └── inspect_memory.rs
│
├── frontend/                        # === FRONTEND (Phase 6) ===
│   ├── package.json
│   ├── package-lock.json
│   ├── tsconfig.json
│   ├── tsconfig.node.json
│   ├── vite.config.ts
│   ├── tailwind.config.ts
│   ├── postcss.config.js
│   ├── index.html
│   ├── .eslintrc.cjs
│   ├── .prettierrc
│   │
│   ├── public/
│   │   └── nexus.svg                # logo
│   │
│   └── src/
│       ├── main.tsx                 # React 18 createRoot
│       ├── App.tsx                  # layout 3 cột: Sidebar | Chat | Tool Panel
│       ├── index.css                # Tailwind directives + base styles
│       │
│       ├── bindings/                # sinh tự động từ src-tauri qua ts_rs
│       │   ├── commands.ts          # kiểu của tất cả #[tauri::command]
│       │   ├── events.ts            # kiểu của tất cả event payload
│       │   └── types.ts             # các struct #[derive(Ts)]
│       │
│       ├── store/                   # === ZUSTAND STORES ===
│       │   ├── chatStore.ts         # messages, streaming, run state
│       │   ├── sessionStore.ts      # session list + active
│       │   ├── toolStore.ts         # tool activity timeline
│       │   ├── approvalStore.ts     # pending approval requests
│       │   ├── memoryStore.ts       # memory list + recall
│       │   ├── schedulerStore.ts    # scheduled jobs
│       │   ├── configStore.ts       # app config
│       │   └── index.ts             # barrel
│       │
│       ├── lib/
│       │   ├── ipc.ts               # wrapper quanh tauri invoke + listen
│       │   ├── markdown.ts          # react-markdown + rehype/remark plugins
│       │   ├── highlight.ts         # shiki config
│       │   └── format.ts            # date/time, file size, etc.
│       │
│       ├── components/
│       │   ├── layout/
│       │   │   ├── Sidebar.tsx
│       │   │   ├── ChatPanel.tsx
│       │   │   ├── ToolPanel.tsx
│       │   │   └── StatusBar.tsx
│       │   │
│       │   ├── chat/
│       │   │   ├── ChatHeader.tsx
│       │   │   ├── MessageList.tsx
│       │   │   ├── MessageBubble.tsx
│       │   │   ├── MessageInput.tsx
│       │   │   ├── StreamingIndicator.tsx
│       │   │   ├── ToolCallBlock.tsx
│       │   │   └── MarkdownRenderer.tsx
│       │   │
│       │   ├── sidebar/
│       │   │   ├── SessionList.tsx
│       │   │   ├── SessionItem.tsx
│       │   │   ├── NewSessionButton.tsx
│       │   │   └── SessionSearch.tsx
│       │   │
│       │   ├── tools/
│       │   │   ├── ToolTimeline.tsx
│       │   │   ├── ToolTimelineItem.tsx
│       │   │   ├── ToolResultViewer.tsx
│       │   │   └── ToolStatusBadge.tsx
│       │   │
│       │   ├── approval/
│       │   │   ├── ApprovalDialog.tsx
│       │   │   └── ApprovalToast.tsx
│       │   │
│       │   ├── memory/
│       │   │   ├── MemoryPanel.tsx
│       │   │   ├── MemoryItem.tsx
│       │   │   └── MemorySearchBar.tsx
│       │   │
│       │   ├── scheduler/
│       │   │   ├── SchedulerPanel.tsx
│       │   │   ├── JobForm.tsx
│       │   │   └── JobList.tsx
│       │   │
│       │   ├── settings/
│       │   │   ├── SettingsModal.tsx
│       │   │   ├── ProviderSection.tsx
│       │   │   ├── SecuritySection.tsx
│       │   │   └── WorkspaceSection.tsx
│       │   │
│       │   └── ui/                  # primitive components (shadcn-style)
│       │       ├── Button.tsx
│       │       ├── Input.tsx
│       │       ├── Textarea.tsx
│       │       ├── Dialog.tsx
│       │       ├── Tooltip.tsx
│       │       ├── ScrollArea.tsx
│       │       ├── Badge.tsx
│       │       ├── Card.tsx
│       │       └── Spinner.tsx
│       │
│       ├── hooks/
│       │   ├── useAgentEvents.ts    # subscribe agent:delta / tool_call / done / error
│       │   ├── useApprovalEvents.ts
│       │   ├── useSchedulerEvents.ts
│       │   ├── useDebounce.ts
│       │   └── useCopyToClipboard.ts
│       │
│       ├── pages/                   # view-level containers (optional routing)
│       │   ├── ChatPage.tsx
│       │   ├── MemoryPage.tsx
│       │   └── SchedulerPage.tsx
│       │
│       └── types/                   # frontend-only types
│           ├── ipc.ts               # re-export từ bindings + alias
│           ├── store.ts
│           └── env.d.ts
│
└── scripts/                         # dev tooling
    ├── dev.sh                       # chạy cả cargo + vite ở watch mode
    ├── build.sh                     # build release
    ├── check.sh                     # cargo fmt + clippy + test
    ├── gen_bindings.sh              # sinh bindings/ qua ts_rs
    └── seed_workspace.sh            # tạo ~/nexus_workspace + config mặc định
```

---

## 2. Quy ước đặt tên & tổ chức

### 2.1 Module Rust

- **Một module = một thư mục** với `mod.rs` chứa `pub use` re-export API công khai. Các file con bên trong không bao giờ `pub` trực tiếp với bên ngoài module.
- **Tên file = snake_case**, dài tối đa 2 từ nếu có thể (`read_file.rs`, `run_command.rs`). File dài hơn 800 dòng phải được tách.
- **Tests nằm cùng module**: `tests.rs` trong từng module cho unit tests; thư mục `tests/` ở root workspace cho integration tests (theo chuẩn Cargo).

### 2.2 Frontend

- **Component = PascalCase** (`MessageBubble.tsx`). Component nguyên thủy (UI primitives) ở `components/ui/`.
- **Store = camelCase + hậu tố `Store`** (`chatStore.ts`), mỗi store một Zustand slice riêng để tránh re-render lan.
- **Hooks = tiền tố `use`** (`useAgentEvents.ts`).
- **Types sinh tự động** từ Rust qua `ts_rs` nằm trong `bindings/`, không bao giờ viết tay trùng lặp.

### 2.3 SQL Migrations

- Đánh số `NNNN_description.sql` (4 chữ số, zero-padded).
- Mỗi migration **idempotent** nếu có thể (`CREATE TABLE IF NOT EXISTS`).
- Không sửa migration đã commit — luôn thêm migration mới để rollback-safe.

---

## 3. Tách biệt crate: lib vs binary

Dự án dùng pattern chuẩn Tauri v2:

| Crate | Vai trò | Entry |
|---|---|---|
| `nexus` (lib crate, ở `src/`) | Toàn bộ logic core: agent, tools, memory, scheduler, llm, db, security, config, commands. Có thể `cargo test` độc lập. | `src/lib.rs` |
| `nexus-app` (binary crate, ở `src-tauri/`) | Khởi tạo `AppState`, đăng ký Tauri commands, launch WebView. Mọi logic đều delegate sang `nexus::`. | `src-tauri/src/main.rs` |

**Lợi ích:**
- Test toàn bộ logic mà không cần chạy Tauri / WebView.
- Có thể viết `examples/run_agent_cli.rs` dùng `nexus::agent::Agent` để debug qua terminal.
- Frontend và backend deploy cùng lúc nhưng build độc lập — Vite dev server có thể chạy riêng.

---

## 4. Cây phụ thuộc nội bộ

```
                ┌─────────────┐
                │  nexus-app  │ (src-tauri/)
                └──────┬──────┘
                       │
                ┌──────▼──────┐
                │    nexus    │ (src/lib.rs)
                └──────┬──────┘
                       │
   ┌──────────┬────────┼─────────┬───────────┐
   │          │        │         │           │
┌──▼───┐ ┌────▼───┐ ┌──▼───┐ ┌───▼────┐ ┌────▼────┐
│agent │ │commands│ │scheduler│ │browser│ │memory   │
└──┬───┘ └────┬───┘ └──┬───┘ └───┬────┘ └────┬────┘
   │          │        │         │           │
   └────┬─────┴────────┴────┬────┴───────────┘
        │                   │
   ┌────▼────┐         ┌────▼────┐
   │  tools  │         │  llm    │
   └────┬────┘         └─────────┘
        │
   ┌────▼────┐
   │security │
   └────┬────┘
        │
   ┌────▼────┐
   │database │
   └─────────┘

config + utils + error + observability: depended-on bởi tất cả
```

**Quy tắc phụ thuộc:**
- `agent` dùng `tools`, `llm`, `memory`, `security`, `database`, `config` — KHÔNG bao giờ ngược lại.
- `tools` dùng `security`, `database`, `browser`, `memory`, `scheduler` (qua `ToolContext`, không trực tiếp import).
- `commands` là lớp mỏng chỉ gọi `agent` / `tools` / `memory` / `scheduler` / `config` — không chứa logic nghiệp vụ.
- Không có phụ thuộc vòng. Clippy rule `clippy::cognitive_complexity` + `deny(missing_docs)` để giữ chất lượng.

---

## 5. Tổ chức migrations SQL

```
src/database/migrations/
├── 0001_init.sql
├── 0002_embedding_dim.sql
└── 0003_indexes.sql
```

- **`0001_init.sql`** — tạo 5 bảng `sessions`, `messages`, `memories`, `tasks`, `command_logs` + index cơ bản.
- **`0002_embedding_dim.sql`** — thêm cột `embedding_dim INTEGER NOT NULL DEFAULT 1536` vào `memories` để hỗ trợ cả OpenAI (1536) và Ollama `nomic-embed-text` (768).
- **`0003_indexes.sql`** — index phụ trợ: `idx_messages_session_created`, `idx_command_logs_session_started`, `idx_memories_last_used`.

SQLx embed toàn bộ thư mục này vào binary lúc compile qua `sqlx::migrate!()`. Không cần file `.sql` runtime.

---

## 6. Tổ chức frontend theo feature

Frontend chia theo **feature folder** kết hợp **role-based**:

```
frontend/src/
├── components/
│   ├── layout/      # frame ngoài (3 cột)
│   ├── chat/        # feature Chat
│   ├── sidebar/     # feature Sessions
│   ├── tools/       # feature Tool Activity
│   ├── approval/    # feature Approval
│   ├── memory/      # feature Memory
│   ├── scheduler/   # feature Scheduler
│   ├── settings/    # feature Settings
│   └── ui/          # primitive (button, dialog, ...)
```

Mỗi feature độc lập, import cross-feature chỉ qua `store/` và `bindings/`. Component nguyên thủy `ui/` không được import component feature.

---

## 7. Tổ chức test

```
src/<module>/tests.rs         # unit test — gọi hàm nội bộ module
tests/                        # integration test — gọi API công khai qua crate root
├── common/mod.rs             # helper: temp_workspace(), mock_provider(), in_memory_db()
├── agent_loop_integration.rs
├── tools_file_integration.rs
├── ...
benches/                      # criterion benchmarks (chạy với --bench)
examples/                     # binary examples để debug thủ công
```

**Quy ước:**
- Unit test không spawn task thật — dùng `tokio::test` cho async.
- Integration test dùng `tempfile::TempDir` cho workspace, `:memory:` SQLite cho db.
- Mock LLM qua struct `MockProvider` implementing `LLMProvider`, trả script cứng.
- Browser test được `#[ignore]` mặc định, chạy bằng `cargo test -- --ignored` khi có Chromium.

---

## 8. Asset & resources

```
icons/                         # Tauri yêu cầu 5 file icon chuẩn
├── 32x32.png
├── 128x128.png
├── 128x128@2x.png
├── icon.icns                  # macOS
└── icon.ico                   # Windows

frontend/public/
└── nexus.svg                  # logo SVG (dùng cho splash + sidebar header)
```

Không có binary asset nào khác trong repo (Chromium/Node/Python được phát hiện ở runtime qua `which`/`where`).

---

## 9. File cấu hình & tooling

| File | Mục đích |
|---|---|
| `Cargo.toml` (root) | Workspace manifest, `[workspace.dependencies]` chung |
| `src-tauri/Cargo.toml` | Binary crate manifest |
| `tauri.conf.json` | Cấu hình Tauri v2: app id, window, bundle, allowlist |
| `capabilities/default.json` | ACL capability cho Tauri v2 (fs, shell, http) |
| `rust-toolchain.toml` | Pin Rust version + components (clippy, rustfmt) |
| `.cargo/config.toml` | Target dir, build profile flags |
| `frontend/tsconfig.json` | TypeScript strict mode |
| `frontend/vite.config.ts` | Vite + React plugin + alias `@` → `src/` |
| `frontend/tailwind.config.ts` | Dark theme, custom colors (nexus palette) |
| `.eslintrc.cjs` / `.prettierrc` | FE lint/format |
| `.editorconfig` |统一 indent 2 spaces, LF line endings |
| `.gitignore` | `target/`, `node_modules/`, `dist/`, `logs/`, `*.db` |

---

## 10. Cây thư mục tạo ở runtime (KHÔNG commit)

```
~/nexus_workspace/
├── nexus.db                    # SQLite database
├── config.toml                 # AppConfig
├── workspace/                  # sandbox root cho file tools
│   └── <files do agent tạo>
└── logs/
    └── app.log                 # rolling daily, 7 ngày retain
```

Đây là **user data dir**, KHÔNG nằm trong repo. Được tạo tự động ở lần chạy đầu qua `config::paths::ensure_workspace()`.

---

## 11. Tổng kết Phase 2

Cấu trúc trên đáp ứng các yêu cầu:

- ✅ Mỗi module (`agent`, `tools`, `memory`, `scheduler`, `browser`, `llm`, `commands`, `database`, `config`, `security`) là thư mục riêng, API công khai rõ ràng.
- ✅ Tách lib crate (test được độc lập) và binary crate (Tauri entrypoint).
- ✅ 24 tool được tổ chức theo category trong `src/tools/<category>/`.
- ✅ Migrations SQL nằm cùng code, embed vào binary.
- ✅ Frontend chia theo feature + có store Zustand riêng từng slice.
- ✅ Test tách unit (trong module) và integration (thư mục `tests/`).
- ✅ Asset & resource có vị trí cố định theo chuẩn Tauri v2.
- ✅ Runtime data tách biệt khỏi repo.

---

**Kết thúc Phase 2.** Chờ xác nhận để tiếp tục **Phase 3 — Cargo.toml** (workspace manifest + dependencies pin version).
