//! NEXUS — Tauri v2 entrypoint.
//!
//! Bootstrap flow:
//!   1. Ensure workspace dirs exist
//!   2. Init tracing
//!   3. Load config
//!   4. Init SQLite pool + run migrations
//!   5. Build AppState
//!   6. Register default tools
//!   7. Start scheduler + restore jobs
//!   8. Register Tauri commands + launch WebView

#![forbid(unsafe_code)]

use std::sync::Arc;

use nexus::{
    config::{paths, ConfigStore, AppConfig},
    database::pool::init_pool,
    observability,
    state::AppState,
};
use tauri::Manager;

fn main() {
    // 1. Workspace dirs
    if let Err(e) = paths::ensure_workspace() {
        eprintln!("FATAL: cannot ensure workspace: {e}");
        std::process::exit(1);
    }

    // 2. Tracing
    let _ = observability::init(paths::log_dir());

    // 3. Load config
    let config_store = ConfigStore::new(paths::config_path());
    let config: AppConfig = match config_store.load_or_init() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "config load failed; using defaults");
            AppConfig::defaults()
        }
    };
    let config = Arc::new(config);

    // 4. DB pool
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build Tokio runtime");
    let pool = rt
        .block_on(async { init_pool(paths::db_path()).await })
        .expect("failed to init DB pool");

    // 5. AppState
    let state = rt
        .block_on(async { AppState::new(pool.clone(), Arc::clone(&config)).await })
        .expect("failed to build AppState");

    // 6. Register tools
    state.register_default_tools();

    // 7. Start scheduler
    let state_for_sched = Arc::clone(&state);
    rt.block_on(async {
        if let Err(e) = state_for_sched.scheduler.start().await {
            tracing::error!(error = %e, "scheduler start failed");
        }
    });

    // 8. Launch Tauri
    tauri::Builder::default()
        .manage(state.clone())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            // chat
            nexus::commands::chat::chat_send,
            nexus::commands::chat::chat_cancel,
            // session
            nexus::commands::session::session_create,
            nexus::commands::session::session_list,
            nexus::commands::session::session_search,
            nexus::commands::session::session_rename,
            nexus::commands::session::session_delete,
            // memory
            nexus::commands::memory::memory_save,
            nexus::commands::memory::memory_recall,
            nexus::commands::memory::memory_list,
            nexus::commands::memory::memory_delete,
            // scheduler
            nexus::commands::scheduler::scheduler_add,
            nexus::commands::scheduler::scheduler_list,
            nexus::commands::scheduler::scheduler_cancel,
            // tool
            nexus::commands::tool::tool_list,
            nexus::commands::tool::tool_invoke,
            // browser
            nexus::commands::browser::browser_action,
            nexus::commands::browser::browser_shutdown,
            // config
            nexus::commands::config::config_get,
            nexus::commands::config::config_set,
            // approval
            nexus::commands::approval::approval_respond,
            nexus::commands::approval::approval_pending,
            // custom providers
            nexus::commands::custom_provider::custom_provider_add,
            nexus::commands::custom_provider::custom_provider_remove,
            nexus::commands::custom_provider::custom_provider_list,
            nexus::commands::custom_provider::provider_list_all,
            nexus::commands::custom_provider::custom_provider_preset,
            // command palette
            nexus::commands::palette::palette_search,
            nexus::commands::file_upload::read_file_for_chat,
        ])
        .setup(|app| {
            tracing::info!("NEXUS v{} started", nexus::VERSION);
            let _ = app.app_handle();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running NEXUS application");

    // Shutdown
    rt.block_on(async {
        let _ = state.scheduler.shutdown().await;
        let _ = state.browser.shutdown().await;
    });
    drop(rt);
}
