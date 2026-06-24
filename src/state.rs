//! NEXUS — global app state, chia sẻ qua `Arc<AppState>`.

use std::sync::Arc;

use dashmap::DashMap;
use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;

use crate::browser::BrowserManager;
use crate::config::AppConfig;
use crate::error::Result;
use crate::memory::store::MemoryStore;
use crate::scheduler::SchedulerService;
use crate::security::approval::ApprovalGate;
use crate::security::Sandbox;
use crate::tools::registry::ToolRegistry;

/// Shared app state — clone via `Arc<AppState>`.
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<AppConfig>,
    pub tool_registry: Arc<ToolRegistry>,
    pub memory: Arc<MemoryStore>,
    pub scheduler: Arc<SchedulerService>,
    pub browser: Arc<BrowserManager>,
    pub sandbox: Arc<Sandbox>,
    pub approval_gate: Arc<ApprovalGate>,
    /// Active agent runs — keyed by run_id. Cancellation token per run.
    pub active_runs: DashMap<String, CancellationToken>,
}

impl AppState {
    /// Khởi tạo toàn bộ state từ config + pool.
    ///
    /// **Resilient**: Nếu embedding provider fail (vd: chưa có API key),
    /// tiếp tục với None embedding → memory features sẽ degraded nhưng app vẫn chạy.
    pub async fn new(pool: SqlitePool, config: Arc<AppConfig>) -> Result<Arc<Self>> {
        // Sandbox
        let sandbox = Arc::new(Sandbox::new_default());

        // Tool registry (empty at this point — register ở main.rs sau khi state tạo)
        let tool_registry = Arc::new(ToolRegistry::new());

        // Approval gate
        let approval_gate = Arc::new(ApprovalGate::new(config.security.approval_timeout_secs));

        // Memory store — embedding provider dùng build_provider_from_app_config
        // (hỗ trợ cả custom OpenAI-compatible providers)
        // Resilient: nếu provider build fail (vd: chưa có API key), dùng None
        // → memory features sẽ degraded nhưng app vẫn chạy
        let memory = match crate::llm::factory::build_provider_from_app_config(
            &config.memory.embedding_provider,
            &config,
            Some(&config.memory.embedding_model),
        ) {
            Ok(embedding_provider) => {
                let embedding_client = Arc::new(crate::memory::embedding::EmbeddingClient::new(embedding_provider));
                Arc::new(MemoryStore::new(
                    pool.clone(),
                    embedding_client,
                    config.memory.dedup_threshold,
                ))
            }
            Err(e) => {
                tracing::warn!(error = %e, "embedding provider init failed — memory features will be degraded");
                // Create a dummy memory store with no embedding — will return empty results
                // For now, we use Ollama as fallback (local, no API key needed)
                let fallback = crate::llm::factory::build_provider_from_app_config(
                    "ollama",
                    &config,
                    Some("nomic-embed-text"),
                );
                match fallback {
                    Ok(p) => {
                        let ec = Arc::new(crate::memory::embedding::EmbeddingClient::new(p));
                        Arc::new(MemoryStore::new(pool.clone(), ec, config.memory.dedup_threshold))
                    }
                    Err(_) => {
                        // Last resort: use ollama provider directly (will fail at embed time, but app runs)
                        let p = crate::llm::ollama::OllamaProvider::new(
                            "http://localhost:11434".into(),
                            "nomic-embed-text".into(),
                        )?;
                        let ec = Arc::new(crate::memory::embedding::EmbeddingClient::new(Arc::new(p)));
                        Arc::new(MemoryStore::new(pool.clone(), ec, config.memory.dedup_threshold))
                    }
                }
            }
        };

        // Browser manager
        let browser = Arc::new(BrowserManager::new(
            config.browser.headless,
            config.browser.port,
        ));

        // Scheduler service — on_fire callback là no-op mặc định (sẽ wire từ main.rs qua event emitter)
        let on_fire: crate::scheduler::service::FireCallback = Arc::new(|_id, _msg, _session| {
            tracing::info!("scheduler fired (default no-op callback)");
        });
        let scheduler = Arc::new(SchedulerService::new(pool.clone(), on_fire));

        Ok(Arc::new(Self {
            pool,
            config,
            tool_registry,
            memory,
            scheduler,
            browser,
            sandbox,
            approval_gate,
            active_runs: DashMap::new(),
        }))
    }

    /// Register toàn bộ default tools. Gọi sau khi AppState đã tạo.
    pub fn register_default_tools(&self) {
        crate::tools::file::register_all(&self.tool_registry);
        crate::tools::shell::register_all(&self.tool_registry);
        crate::tools::code::register_all(&self.tool_registry);
        crate::tools::browser::register_all(&self.tool_registry);
        crate::tools::search::register_all(&self.tool_registry);
        crate::tools::memory::register_all(&self.tool_registry);
        crate::tools::scheduler::register_all(&self.tool_registry);
        tracing::info!(count = self.tool_registry.len(), "all default tools registered");
    }
}
