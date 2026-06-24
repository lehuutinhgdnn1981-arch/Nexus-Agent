//! Example: run the agent from CLI (no UI). Useful for debugging.
//!
//! Run:
//!   OPENAI_API_KEY=sk-... cargo run --example run_agent_cli -- \
//!     --session test --message "list files in current directory"

use std::sync::Arc;

use clap::{Parser, ValueEnum};
use nexus::agent::agent::Agent;
use nexus::agent::config::AgentRuntimeConfig;
use nexus::agent::event::AgentEvent;
use nexus::config::paths;
use nexus::config::ConfigStore;
use nexus::database::pool::init_pool;
use nexus::llm::factory::build_provider;
use nexus::observability;
use nexus::state::AppState;
use tokio::sync::mpsc;

#[derive(Debug, Clone, ValueEnum)]
enum ProviderArg {
    Openai,
    Openrouter,
    Anthropic,
    Ollama,
}

#[derive(Parser, Debug)]
#[command(name = "run_agent_cli", about = "Run NEXUS agent from CLI")]
struct Args {
    /// Session ID (created if not exists)
    #[arg(long, default_value = "cli_session")]
    session: String,

    /// User message to send
    #[arg(long)]
    message: String,

    /// LLM provider
    #[arg(long, value_enum, default_value_t = ProviderArg::Openai)]
    provider: ProviderArg,

    /// Model name
    #[arg(long)]
    model: Option<String>,

    /// Max iterations
    #[arg(long, default_value_t = 10)]
    max_iterations: u32,

    /// Verbose
    #[arg(long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Init
    paths::ensure_workspace()?;
    let _ = observability::init_stdout();

    let config_store = ConfigStore::new(paths::config_path());
    let config = Arc::new(config_store.load_or_init()?);

    let pool = init_pool(paths::db_path()).await?;
    let state = AppState::new(pool, Arc::clone(&config)).await?;
    state.register_default_tools();

    let provider_name = match args.provider {
        ProviderArg::Openai => "openai",
        ProviderArg::Openrouter => "openrouter",
        ProviderArg::Anthropic => "anthropic",
        ProviderArg::Ollama => "ollama",
    };

    let provider_cfg = config.provider(provider_name).ok_or_else(|| {
        anyhow::anyhow!("provider `{provider_name}` not configured")
    })?;
    let model = args
        .model
        .clone()
        .or_else(|| provider_cfg.default_model.clone())
        .unwrap_or_else(|| "gpt-4o-mini".into());

    let provider = build_provider(provider_name, provider_cfg, &config.memory.embedding_model)?;

    let mut rt_config = AgentRuntimeConfig::default();
    rt_config.default_provider = provider_name.into();
    rt_config.default_model = model.clone();
    rt_config.max_iterations = args.max_iterations;
    rt_config.max_tool_calls = 50;

    // Create session if not exists
    use nexus::database::repositories::session_repo::SessionRepo;
    if SessionRepo::get(&state.pool, &args.session).await.is_err() {
        SessionRepo::create(
            &state.pool,
            &args.session,
            "CLI Session",
            provider_name,
            &model,
            None,
        )
        .await?;
        println!("[created session `{}`]", args.session);
    }

    let agent = Agent::new(Arc::clone(&state), rt_config, provider);
    let (tx, mut rx) = mpsc::channel::<AgentEvent>(128);

    println!();
    println!("════════════════════════════════════════════════════════════════");
    println!("User: {}", args.message);
    println!("════════════════════════════════════════════════════════════════");
    println!();

    let agent_handle = tokio::spawn(async move {
        if let Err(e) = agent.run(&args.session, &args.message, tx).await {
            eprintln!("[agent error: {e}]");
        }
    });

    while let Some(ev) = rx.recv().await {
        match ev {
            AgentEvent::TurnStart { run_id, .. } => {
                if args.verbose {
                    println!("[turn start run_id={run_id}]");
                }
            }
            AgentEvent::Delta { text, .. } => {
                print!("{text}");
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
            AgentEvent::ToolCallStart { tool, input, .. } => {
                println!();
                println!("→ tool: {tool}");
                if args.verbose {
                    println!("  input: {input}");
                }
            }
            AgentEvent::ToolCallEnd { result, .. } => {
                if args.verbose {
                    println!("  result ok={} ({} chars)", result.ok, result.output.len());
                    if !result.ok {
                        println!("  error: {}", result.output);
                    }
                }
            }
            AgentEvent::IterationEnd { iteration, tool_calls_made, .. } => {
                if args.verbose {
                    println!("[iteration {iteration} done, {tool_calls_made} tool calls]");
                }
            }
            AgentEvent::ApprovalRequest { tool, input, .. } => {
                println!();
                println!("⚠️  APPROVAL REQUIRED: {tool}");
                println!("   input: {input}");
                println!("   (approve via UI in real app — auto-approving in CLI mode)");
                // For CLI example, auto-approve. In real app this goes via IPC.
            }
            AgentEvent::Done { final_message, .. } => {
                println!();
                println!();
                println!("════════════════════════════════════════════════════════════════");
                println!("[done]");
                if !final_message.is_empty() && args.verbose {
                    println!("Final: {final_message}");
                }
                break;
            }
            AgentEvent::Error { message, .. } => {
                println!();
                eprintln!("[ERROR] {message}");
                break;
            }
            AgentEvent::Cancelled { .. } => {
                println!();
                println!("[cancelled]");
                break;
            }
        }
    }

    agent_handle.await?;
    Ok(())
}
