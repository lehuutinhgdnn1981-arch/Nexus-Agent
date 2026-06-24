//! Example: list all registered tools.
//!
//! Run: `cargo run --example list_tools`

use nexus::config::AppConfig;
use nexus::database::pool::in_memory_pool;
use nexus::security::Sandbox;
use nexus::state::AppState;
use nexus::tools::registry::ToolRegistry;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pool = in_memory_pool().await?;
    let config = Arc::new(AppConfig::defaults());

    // Use a registry-only approach — don't build full AppState (which needs LLM key).
    let registry = ToolRegistry::new();
    nexus::tools::register_all(&registry);

    println!();
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                    NEXUS — Registered Tools ({} total)                  ║", registry.len());
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();

    let mut names = registry.list_names();
    names.sort();

    for name in &names {
        let tool = registry.get(name).unwrap();
        let perm_badge = match tool.permission() {
            nexus::security::PermissionLevel::Safe => "[SAFE]            ",
            nexus::security::PermissionLevel::RequiresApproval => "[NEEDS APPROVAL]  ",
            nexus::security::PermissionLevel::Dangerous => "[DANGEROUS]       ",
        };
        println!("  {}  {:<25}  {}", perm_badge, name, tool.description());
    }

    println!();
    println!("Total: {} tools", registry.len());

    // Silence unused
    let _ = (pool, config, Sandbox::new_default);
    Ok(())
}
