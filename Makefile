# NEXUS — Makefile

.PHONY: help build dev test test-unit test-integration test-browser lint clippy fmt clean frontend-install frontend-dev frontend-build coverage run-cli list-tools inspect

RUST_FEATURES = --features test-utils

help:
	@echo "NEXUS — Make targets"
	@echo ""
	@echo "Setup:"
	@echo "  frontend-install   Install frontend dependencies"
	@echo ""
	@echo "Build:"
	@echo "  build              Build release binary"
	@echo "  frontend-build     Build frontend (Vite production)"
	@echo ""
	@echo "Development:"
	@echo "  dev                Start Tauri dev mode (hot reload)"
	@echo "  frontend-dev       Start Vite dev server only"
	@echo ""
	@echo "Tests:"
	@echo "  test               Run all tests (unit + integration)"
	@echo "  test-unit          Run unit tests only"
	@echo "  test-integration   Run integration tests only"
	@echo "  test-browser       Run browser smoke tests (needs Chromium)"
	@echo ""
	@echo "Quality:"
	@echo "  lint               Run clippy with -D warnings"
	@echo "  fmt                Format all code"
	@echo "  coverage           Generate coverage report"
	@echo ""
	@echo "Examples:"
	@echo "  list-tools         List all registered tools"
	@echo "  run-cli            Run agent from CLI (set OPENAI_API_KEY env)"
	@echo "  inspect            Inspect memory + scheduler + command logs"
	@echo ""
	@echo "Clean:"
	@echo "  clean              Remove target/ + frontend/dist/ + node_modules/"

# === Setup ===
frontend-install:
	cd frontend && npm install

# === Build ===
build:
	cargo build --workspace --release

frontend-build:
	cd frontend && npm run build

# === Dev ===
dev:
	cd src-tauri && cargo tauri dev

frontend-dev:
	cd frontend && npm run dev

# === Tests ===
test:
	cargo test --workspace $(RUST_FEATURES)

test-unit:
	cargo test --workspace --lib $(RUST_FEATURES)

test-integration:
	cargo test --workspace --test '*' $(RUST_FEATURES)

test-browser:
	cargo test --test browser_smoke $(RUST_FEATURES) -- --ignored

# === Quality ===
lint:
	cargo clippy --workspace --all-targets $(RUST_FEATURES) -- -D warnings

fmt:
	cargo fmt --all
	cd frontend && npx prettier --write "src/**/*.{ts,tsx,css}" || true

coverage:
	cargo tarpaulin --workspace $(RUST_FEATURES) --out Html --output-dir coverage/ --skip-clean

# === Examples ===
list-tools:
	cargo run --example list_tools --features examples

run-cli:
	cargo run --example run_agent_cli --features examples -- $(ARGS)

inspect:
	cargo run --example inspect_memory --features examples

# === Clean ===
clean:
	cargo clean
	rm -rf frontend/dist frontend/node_modules
	rm -rf coverage
