//! Shared test fixtures.

use std::sync::Arc;

use nexus::browser::BrowserManager;
use nexus::config::AppConfig;
use nexus::database::pool::in_memory_pool;
use nexus::llm::factory::build_provider;
use nexus::llm::MockProvider;
use nexus::memory::embedding::EmbeddingClient;
use nexus::memory::store::MemoryStore;
use nexus::scheduler::SchedulerService;
use nexus::security::Sandbox;
use nexus::state::AppState;
use nexus::tools::context::ToolContext;
use sqlx::SqlitePool;

/// Tạo AppState với in-memory DB + temp workspace.
pub async fn app_state(_temp_dir: &std::path::Path) -> Arc<AppState> {
    let pool: SqlitePool = in_memory_pool().await.expect("pool");
    let mut config = AppConfig::defaults();
    config.memory.embedding_provider = "ollama".into();
    let config = Arc::new(config);

    let state = AppState::new(pool, Arc::clone(&config))
        .await
        .expect("AppState");

    state.register_default_tools();
    state
}

/// Tạo `ToolContext` dùng temp workspace + in-memory DB + Ollama (fallback) provider.
pub async fn tool_context(temp_dir: &std::path::Path) -> ToolContext {
    let pool = in_memory_pool().await.expect("pool");
    let config = Arc::new(AppConfig::defaults());
    let sandbox = Arc::new(Sandbox::new(temp_dir.to_path_buf()));

    let provider = build_provider(
        "ollama",
        config.provider("ollama").unwrap(),
        &config.memory.embedding_model,
    )
    .expect("ollama provider");

    let embedding_client = Arc::new(EmbeddingClient::new(provider));
    let memory = Arc::new(MemoryStore::new(pool.clone(), embedding_client, 0.92));
    let browser = Arc::new(BrowserManager::new(true, 9222));
    let on_fire: nexus::scheduler::service::FireCallback = Arc::new(|_, _, _| {});
    let scheduler = Arc::new(SchedulerService::new(pool.clone(), on_fire));

    ToolContext {
        session_id: Some("test_session".into()),
        run_id: Some("test_run".into()),
        workspace: sandbox,
        pool,
        memory,
        browser,
        scheduler,
        config,
    }
}

/// Tạo ToolContext với Mock LLM provider (cho agent loop tests).
pub async fn tool_context_with_mock(
    temp_dir: &std::path::Path,
    mock: Arc<MockProvider>,
) -> ToolContext {
    let pool = in_memory_pool().await.expect("pool");
    let config = Arc::new(AppConfig::defaults());
    let sandbox = Arc::new(Sandbox::new(temp_dir.to_path_buf()));

    let embedding_client = Arc::new(EmbeddingClient::new(mock));
    let memory = Arc::new(MemoryStore::new(pool.clone(), embedding_client, 0.92));
    let browser = Arc::new(BrowserManager::new(true, 9222));
    let on_fire: nexus::scheduler::service::FireCallback = Arc::new(|_, _, _| {});
    let scheduler = Arc::new(SchedulerService::new(pool.clone(), on_fire));

    ToolContext {
        session_id: Some("test_session".into()),
        run_id: Some("test_run".into()),
        workspace: sandbox,
        pool,
        memory,
        browser,
        scheduler,
        config,
    }
}

/// Build AppState với mock embedding provider (cho agent loop tests).
pub async fn app_state_with_mock(
    temp_dir: &std::path::Path,
    mock: Arc<MockProvider>,
) -> Arc<AppState> {
    use dashmap::DashMap;
    use nexus::security::approval::ApprovalGate;
    use nexus::tools::registry::ToolRegistry;

    let pool = in_memory_pool().await.expect("pool");
    let config = Arc::new(AppConfig::defaults());

    let embedding_client = Arc::new(EmbeddingClient::new(mock));
    let memory = Arc::new(MemoryStore::new(pool.clone(), embedding_client, 0.92));
    let sandbox = Arc::new(Sandbox::new(temp_dir.to_path_buf()));
    let tool_registry = Arc::new(ToolRegistry::new());
    let approval_gate = Arc::new(ApprovalGate::new(config.security.approval_timeout_secs));
    let browser = Arc::new(BrowserManager::new(true, 9222));
    let on_fire: nexus::scheduler::service::FireCallback = Arc::new(|_, _, _| {});
    let scheduler = Arc::new(SchedulerService::new(pool.clone(), on_fire));

    let state = Arc::new(AppState {
        pool,
        config,
        tool_registry,
        memory,
        scheduler,
        browser,
        sandbox,
        approval_gate,
        active_runs: DashMap::new(),
    });
    state.register_default_tools();
    state
}
