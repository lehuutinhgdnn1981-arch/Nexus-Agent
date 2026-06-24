//! Example: inspect memory contents.
//!
//! Run: `cargo run --example inspect_memory`

use nexus::config::paths;
use nexus::config::ConfigStore;
use nexus::database::pool::init_pool;
use nexus::database::repositories::memory_repo::MemoryRepo;
use nexus::database::repositories::task_repo::TaskRepo;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    paths::ensure_workspace()?;
    let config_store = ConfigStore::new(paths::config_path());
    let _config = config_store.load_or_init()?;

    let pool = init_pool(paths::db_path()).await?;

    println!();
    println!("╔════════════════════════════════════════════════╗");
    println!("║          NEXUS — Memory Inspector              ║");
    println!("╚════════════════════════════════════════════════╝");
    println!();

    // Memories
    let memories = MemoryRepo::list(&pool, 100).await?;
    println!("── Memories ({}) ──────────────────────────────", memories.len());
    for m in &memories {
        println!();
        println!("  • [{}/{}] {} (used {}×)",
            m.category,
            m.id.chars().take(8).collect::<String>(),
            m.content.chars().take(80).collect::<String>(),
            m.use_count,
        );
        let tags: Vec<String> = serde_json::from_str(&m.tags).unwrap_or_default();
        if !tags.is_empty() {
            println!("    tags: {}", tags.join(", "));
        }
        println!("    created: {}  last_used: {}",
            nexus::utils::time::relative(m.created_at),
            nexus::utils::time::relative(m.last_used_at),
        );
    }

    println!();
    println!("── Scheduled Tasks ─────────────────────────────");
    let tasks = TaskRepo::list_all(&pool).await?;
    if tasks.is_empty() {
        println!("  (none)");
    } else {
        for t in &tasks {
            println!();
            println!("  • [{}/{}] kind={}", t.id.chars().take(8).collect::<String>(), t.kind, t.kind);
            if let Some(cron) = &t.cron {
                println!("    cron: {cron}");
            }
            if let Some(fire_at) = t.fire_at {
                println!("    fire_at: {} (in {}s)",
                    fire_at,
                    fire_at.saturating_sub(nexus::utils::time::now_ts()),
                );
            }
            println!("    enabled: {}", t.enabled);
            println!("    payload: {}", t.payload);
        }
    }

    println!();
    println!("── Recent Command Logs ─────────────────────────");
    use nexus::database::repositories::command_log_repo::CommandLogRepo;
    let logs = CommandLogRepo::list_recent(&pool, 10).await?;
    if logs.is_empty() {
        println!("  (none)");
    } else {
        for l in &logs {
            println!("  • [{}] {} — {}",
                l.status,
                l.command.chars().take(60).collect::<String>(),
                nexus::utils::time::relative(l.started_at),
            );
        }
    }

    println!();
    Ok(())
}
