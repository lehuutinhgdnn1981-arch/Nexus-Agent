# NEXUS — Phase 3: Cargo.toml & Tooling

> Setup workspace Cargo, pin version toàn bộ dependency, cấu hình Tauri v2, lints, profiles, và tooling.

---

## 1. Cấu trúc manifest đã tạo

```
nexus/
├── Cargo.toml                    # Workspace root + [workspace.dependencies]
├── rust-toolchain.toml           # Pin Rust 1.81.0
├── .cargo/config.toml            # SQLX_OFFLINE=true, RUST_LOG default
├── .gitignore                    # target/, node_modules/, *.db, logs/, .env
├── .editorconfig                 # UTF-8, LF, 4-space cho Rust, 2-space cho FE
├── src/
│   ├── Cargo.toml                # Lib crate `nexus` (logic core)
│   └── lib.rs                    # Stub Phase 3 (Phase 4 thay toàn bộ)
├── src-tauri/
│   ├── Cargo.toml                # Binary crate `nexus-app` (Tauri entrypoint)
│   ├── build.rs                  # tauri_build::build()
│   ├── tauri.conf.json           # Tauri v2 config (window, bundle, security)
│   └── src/
│       └── main.rs               # Stub Phase 3
├── capabilities/
│   └── default.json              # Tauri v2 capability ACL
├── benches/                      # criterion benches (stub Phase 3)
│   ├── agent_loop.rs
│   └── cosine_search.rs
├── examples/                     # CLI debug tools (stub Phase 3)
│   ├── run_agent_cli.rs
│   ├── list_tools.rs
│   └── inspect_memory.rs
├── frontend/
│   └── package.json              # React 18 + Vite + Tailwind + Zustand
└── tests/                        # Integration tests (Phase 7)
```

---

## 2. Workspace Cargo.toml — điểm chính

### 2.1 Members

```toml
[workspace]
resolver = "2"
members  = ["src", "src-tauri"]
```

- `src/` là lib crate `nexus` — toàn bộ logic core, có thể `cargo test --lib` độc lập.
- `src-tauri/` là binary crate `nexus-app` — Tauri v2 entrypoint, delegate mọi logic sang `nexus::`.

### 2.2 Shared dependencies (pin version nhất quán)

Tất cả dependencies dùng chung khai báo ở `[workspace.dependencies]`, các crate con dùng `dep.workspace = true`. Tổng kết các version chính:

| Category | Crate | Version |
|---|---|---|
| Async runtime | `tokio` | 1.40 (full) |
| Async runtime | `tokio-util` | 0.7 |
| Async runtime | `tokio-stream` | 0.1 |
| Async runtime | `futures` | 0.3 |
| Async runtime | `async-trait` | 0.1 |
| HTTP | `reqwest` | 0.12 (json + stream + rustls-tls) |
| HTTP | `reqwest-eventsource` | 0.6 (SSE cho LLM streaming) |
| HTTP | `url` | 2.5 |
| Tauri | `tauri` | 2.0 |
| Tauri plugins | `tauri-plugin-{shell,fs,dialog,os,process,store}` | 2.0 |
| Tauri | `tauri-build` | 2.0 |
| DB | `sqlx` | 0.8 (sqlite + rustls + macros + migrate + chrono + uuid) |
| Serialization | `serde` | 1.0 |
| Serialization | `serde_json` | 1.0 |
| Serialization | `toml` | 0.8 |
| Error | `thiserror` | 1.0 |
| Error | `anyhow` | 1.0 |
| Tracing | `tracing` | 0.1 |
| Tracing | `tracing-subscriber` | 0.3 (env-filter + fmt + json + time) |
| Tracing | `tracing-appender` | 0.2 (rolling daily) |
| Time | `chrono` | 0.4 (serde) |
| UUID | `uuid` | 1.10 (v4 + serde) |
| Concurrency | `dashmap` | 6.1 |
| Concurrency | `parking_lot` | 0.12 |
| Concurrency | `once_cell` | 1.20 |
| Misc | `bytes`, `base64`, `hex`, `sha2`, `regex`, `glob`, `walkdir`, `which`, `dirs`, `humantime`, `indexmap` | latest |
| Browser | `chromiumoxide` | 0.6 (tokio-runtime) |
| Scheduler | `tokio-cron-scheduler` | 0.11 |
| TS bindings | `ts_rs` | 0.9 (chrono-impl) |
| Testing | `mockall` | 0.13 |
| Testing | `pretty_assertions` | 1.4 |
| Testing | `proptest` | 1.5 |
| Testing | `criterion` | 0.5 (html_reports) |

### 2.3 Lint policy (strict)

```toml
[workspace.lints.rust]
unsafe_code     = "deny"
missing_docs    = "warn"
unused_must_use = "deny"
rust_2018_idioms = { level = "deny", priority = -1 }

[workspace.lints.clippy]
all                  = "deny"
pedantic             = "warn"
nursery              = "warn"
unwrap_used          = "deny"
expect_used          = "deny"
panic                = "deny"
dbg_macro            = "deny"
print_stdout         = "warn"
print_stderr         = "warn"
cognitive_complexity = "warn"
```

→ Đảm bảo không có `unwrap()` / `expect()` / `panic!()` trong production code. CI sẽ fail nếu có.

### 2.4 Profiles

- **`[profile.release]`**: `opt-level=3`, `lto=fat`, `codegen-units=1`, `strip=symbols`, `panic=abort` — binary nhỏ, chạy nhanh.
- **`[profile.dev]`**: `opt-level=0`, `debug=true`, `incremental=true` — build nhanh khi dev.
- **`[profile.dev.package."*"]`**: `opt-level=2` — dependency compile tối ưu, code workspace vẫn debug nhanh.
- **`[profile.test]`**: `opt-level=1` — cân bằng tốc độ test và compile.
- **`[profile.bench]`**: `opt-level=3`, `lto=thin` — benchmark chính xác.

---

## 3. Lib crate `src/Cargo.toml`

```toml
[package]
name = "nexus"

[lib]
name = "nexus"
path = "lib.rs"
doctest = true
```

- Mọi dependency dùng `.workspace = true`.
- `[features]`:
  - `default = []`
  - `test-utils = ["dep:mockall", "dep:tempfile"]` — bật mock cho integration test.
  - `ollama-embed = []` — feature dự phòng cho embedding Ollama.
- `[dev-dependencies]`: `mockall`, `pretty_assertions`, `proptest`, `tempfile`, `tokio` (với `test-util` + `macros`).
- 2 benchmarks (criterion, `harness = false`): `cosine_search`, `agent_loop`.
- 3 examples: `run_agent_cli`, `list_tools`, `inspect_memory`.

---

## 4. Binary crate `src-tauri/Cargo.toml`

```toml
[package]
name = "nexus-app"

[dependencies]
nexus = { path = "../src" }     # lib crate

# Tauri v2 + 6 plugins
tauri                  = { workspace = true }
tauri-plugin-shell     = { workspace = true }
tauri-plugin-fs        = { workspace = true }
tauri-plugin-dialog    = { workspace = true }
tauri-plugin-os        = { workspace = true }
tauri-plugin-process   = { workspace = true }
tauri-plugin-store     = { workspace = true }

[build-dependencies]
tauri-build = { workspace = true }

[[bin]]
name = "nexus"
path = "src/main.rs"
```

- KHÔNG chứa logic nghiệp vụ — chỉ bootstrap và delegate sang `nexus::`.
- `[features]`: `default = ["custom-protocol"]` (production), `dev = []` (Vite dev server).

---

## 5. Tauri v2 config (`src-tauri/tauri.conf.json`)

```jsonc
{
  "productName": "NEXUS",
  "version": "0.1.0",
  "identifier": "ai.nexus.desktop",
  "build": {
    "frontendDist": "../frontend/dist",
    "devUrl": "http://localhost:5173",
    "beforeDevCommand": "cd ../frontend && npm run dev",
    "beforeBuildCommand": "cd ../frontend && npm run build"
  },
  "app": {
    "windows": [{
      "title": "NEXUS — Desktop AI Agent",
      "width": 1400,
      "height": 900,
      "minWidth": 1000,
      "minHeight": 700,
      "theme": "Dark"
    }],
    "security": {
      "assetProtocol": {
        "enable": true,
        "scope": ["$HOME/nexus_workspace/**"]
      }
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [...5 icons...]
  }
}
```

- Window tối thiểu 1000×700, dark theme mặc định.
- Asset protocol scope giới hạn trong `~/nexus_workspace/`.
- Bundle sinh tất cả installer (`.msi`, `.dmg`, `.AppImage`).

---

## 6. Capability ACL (`capabilities/default.json`)

Tauri v2 dùng capability-based permission. File `default.json` cấp cho cửa sổ `main` các permission:

- `core:default` + các `core:*:default` — API lõi Tauri.
- `shell:allow-open` — mở URL/file bên ngoài qua shell mặc định.
- `fs:*` — read/write/create-dir/remove/rename/copy/exists.
- `dialog:allow-open`, `dialog:allow-save`.
- `os:default`, `process:default`.

**Lưu ý:** Tool filesystem của agent KHÔNG đi qua Tauri FS plugin mà qua Rust core trực tiếp (để có sandbox riêng). Tauri FS plugin chỉ dùng cho frontend khi cần dialog file picker.

---

## 7. `rust-toolchain.toml`

```toml
[toolchain]
channel    = "1.81.0"
components = ["rustfmt", "clippy", "rust-src"]
profile    = "default"
targets    = ["x86_64-unknown-linux-gnu"]
```

Pin Rust 1.81.0 để đảm bảo mọi dev dùng cùng compiler, có `rustfmt` + `clippy`.

---

## 8. `.cargo/config.toml`

```toml
[build]
jobs = 8

[env]
RUST_LOG = "nexus=info,tauri=info"
SQLX_OFFLINE = "true"

[profile.dev.package."*"]
opt-level = 2
```

- `SQLX_OFFLINE=true` — sử dụng `sqlx-data.json` cache, không cần DB live lúc compile.
- Dependency tối ưu level 2 ngay cả build dev → runtime nhanh hơn nhiều mà không ảnh hưởng compile time của code workspace.

---

## 9. Frontend `package.json` (preview Phase 6)

Dependencies chính:

| Loại | Package | Version |
|---|---|---|
| Tauri API | `@tauri-apps/api`, `@tauri-apps/plugin-{shell,fs,dialog,store}` | ^2.0.0 |
| React | `react`, `react-dom` | ^18.3.1 |
| Markdown | `react-markdown`, `remark-gfm`, `rehype-highlight`, `rehype-raw`, `highlight.js` | latest |
| State | `zustand` | ^4.5.4 |
| Utils | `date-fns`, `clsx`, `lucide-react` | latest |

DevDependencies: `@tauri-apps/cli`, `typescript`, `vite`, `tailwindcss`, `eslint`, `prettier`, `postcss`, `autoprefixer`.

---

## 10. File stubs (Phase 3 → sẽ thay ở Phase 4-7)

Để workspace có thể compile được ở cuối Phase 3, đã tạo các stub:

- `src/lib.rs` — chỉ export `pub const VERSION`.
- `src-tauri/src/main.rs` — `println!` stub.
- `src-tauri/build.rs` — `tauri_build::build()`.
- `benches/*.rs` — criterion stub (Phase 7).
- `examples/*.rs` — `println!` stub (Phase 4/5).

Toàn bộ sẽ được thay bằng code thực ở Phase 4 trở đi.

---

## 11. Verdict

Phase 3 hoàn tất. Workspace sẵn sàng:

- `cargo build` sẽ compile được (với stubs).
- `cargo test --workspace` chạy được (chưa có test thực).
- `cargo clippy --workspace -- -D warnings` pass (vì stubs đơn giản).
- Tauri config + capabilities đầy đủ để chạy `npm run tauri dev` ở Phase 6.

---

**Kết thúc Phase 3.** Chờ xác nhận để tiếp tục **Phase 4 — Backend Core** (agent loop, LLM traits, memory, scheduler, db, config, security, commands).
