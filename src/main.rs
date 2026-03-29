use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

/// BMUX — terminal multiplexer with AI agent orchestration.
#[derive(Parser, Debug)]
#[command(name = "bmux", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// [internal] Run as session daemon (not for direct user invocation)
    #[arg(long, hide = true)]
    daemon: bool,

    /// [internal] Session name when running as daemon
    #[arg(long, hide = true)]
    session_name: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create a new named session and attach to it
    New {
        /// Session name
        name: String,
    },
    /// Attach to an existing session
    Attach {
        /// Session name
        #[arg(short = 's')]
        session: String,
    },
    /// List all active sessions
    Ls,
    /// Kill a session and all its processes
    Kill {
        /// Session name
        #[arg(short = 's')]
        session: String,
    },

    // ── Agent management (Epic 2) ──────────────────────────────────────────
    /// Manage AI agents (spawn, list, status, kill)
    Agent {
        #[command(subcommand)]
        action: AgentCommands,
    },

    // ── Task routing (Epic 3) ──────────────────────────────────────────────
    /// Send and manage tasks for agents
    Task {
        #[command(subcommand)]
        action: TaskCommands,
    },

    // ── Shared context (Epic 4) ────────────────────────────────────────────
    /// Read/write shared context store
    Context {
        #[command(subcommand)]
        action: ContextCommands,
    },

    // ── Workflow orchestration (Epic 5) ────────────────────────────────────
    /// Run and manage multi-agent workflows
    Workflow {
        #[command(subcommand)]
        action: WorkflowCommands,
    },

    // ── Configuration (Story 1.8) ──────────────────────────────────────────
    /// Show or validate configuration
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },

    // ── Data management (Story 6.3) ────────────────────────────────────────
    /// Manage local BMUX data
    Data {
        #[command(subcommand)]
        action: DataCommands,
    },
}

// ── Agent subcommands ─────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
enum AgentCommands {
    /// Spawn an AI agent
    Spawn {
        /// Agent type (claude-code, opencode, pi, shell, custom)
        #[arg(long = "type")]
        agent_type: String,
        /// Agent name
        #[arg(long)]
        name: String,
        /// Model override
        #[arg(long)]
        model: Option<String>,
    },
    /// List all active agents
    List,
    /// Show detailed agent status
    Status {
        /// Agent name
        name: String,
    },
    /// Kill a specific agent
    Kill {
        /// Agent name
        name: String,
    },
}

// ── Task subcommands ──────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
enum TaskCommands {
    /// Send a task to a specific agent or auto-route
    Send {
        /// Agent name (omit with --auto for auto-routing)
        agent: Option<String>,
        /// Task content
        #[arg(last = true)]
        content: String,
        /// Auto-route to best available agent
        #[arg(long)]
        auto: bool,
        /// Prefer a specific model
        #[arg(long)]
        model: Option<String>,
    },
    /// List all tasks
    List,
    /// Cancel a queued task
    Cancel {
        /// Task ID
        id: String,
    },
    /// Show task status
    Status {
        /// Task ID
        id: String,
    },
}

// ── Context subcommands ───────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
enum ContextCommands {
    /// Set a context key-value pair
    Set {
        /// Key
        key: String,
        /// Value
        value: String,
    },
    /// Get a context value by key
    Get {
        /// Key
        key: String,
    },
    /// List all context keys
    List,
    /// Dump entire context as JSON
    Dump,
}

// ── Workflow subcommands ──────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
enum WorkflowCommands {
    /// Run a workflow YAML file
    Run {
        /// Path to workflow YAML
        file: String,
        /// Input parameters as JSON
        #[arg(long)]
        input: Option<String>,
        /// Validate only, don't execute
        #[arg(long)]
        dry_run: bool,
    },
    /// List available workflows
    List,
    /// Show workflow execution status
    Status {
        /// Workflow run ID
        id: String,
    },
}

// ── Config subcommands ────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
enum ConfigCommands {
    /// Display current active configuration
    Show,
    /// Validate config file without applying
    Validate,
}

// ── Data subcommands ──────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
enum DataCommands {
    /// Delete all local BMUX data (audit logs, context, sessions)
    Purge,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Main
// ═══════��═══════════════════════════════════════════════════════════════════════

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("BMUX_LOG").unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    // Internal daemon mode
    if cli.daemon {
        let name = cli.session_name.unwrap_or_else(|| "unnamed".to_string());
        return run_daemon(&name).await;
    }

    match cli.command {
        None => bmux::tui::run_local("local").await,
        Some(Commands::New { name }) => cmd_new(&name).await,
        Some(Commands::Attach { session }) => cmd_attach(&session).await,
        Some(Commands::Ls) => cmd_ls(),
        Some(Commands::Kill { session }) => cmd_kill(&session),
        Some(Commands::Agent { action }) => cmd_agent(action).await,
        Some(Commands::Task { action }) => cmd_task(action).await,
        Some(Commands::Context { action }) => cmd_context(action),
        Some(Commands::Workflow { action }) => cmd_workflow(action).await,
        Some(Commands::Config { action }) => cmd_config(action),
        Some(Commands::Data { action }) => cmd_data(action),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Session commands (Epic 1)
// ═══════════════════════════════════════════════════════════════════════════════

async fn cmd_new(name: &str) -> Result<()> {
    use bmux::tui::session::{list_sessions, socket_path};

    let existing = list_sessions();
    if existing.iter().any(|s| s.name == name) {
        eprintln!("Session '{}' is already running. Use `bmux attach -s {}` to attach.", name, name);
        std::process::exit(1);
    }

    let exe = std::env::current_exe()?;
    let _child = std::process::Command::new(&exe)
        .args(["--daemon", "--session-name", name])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    let sock = socket_path(name);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        if sock.exists() { break; }
        if std::time::Instant::now() >= deadline {
            anyhow::bail!("Timed out waiting for session '{}' to start", name);
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    bmux::tui::client::run_client(sock).await
}

async fn cmd_attach(name: &str) -> Result<()> {
    use bmux::tui::session::socket_path;
    let sock = socket_path(name);
    if !sock.exists() {
        anyhow::bail!("No session named '{}' is running.", name);
    }
    bmux::tui::client::run_client(sock).await
}

fn cmd_ls() -> Result<()> {
    use bmux::tui::session::list_sessions;
    let sessions = list_sessions();
    if sessions.is_empty() {
        println!("No active bmux sessions.");
    } else {
        println!("{:<20} {:>8}  CREATED", "NAME", "WINDOWS");
        println!("{}", "-".repeat(50));
        for s in &sessions {
            println!("{:<20} {:>8}  {}", s.name, s.window_count, s.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
        }
    }
    Ok(())
}

fn cmd_kill(name: &str) -> Result<()> {
    use bmux::tui::session::{list_sessions, meta_path, socket_path};
    let sessions = list_sessions();
    let meta = sessions.iter().find(|s| s.name == name)
        .ok_or_else(|| anyhow::anyhow!("No session named '{}'", name))?;
    send_sigterm(meta.pid);
    let _ = std::fs::remove_file(socket_path(name));
    let _ = std::fs::remove_file(meta_path(name));
    println!("Session '{}' killed.", name);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Agent commands (Epic 2)
// ═══════════════════════════════════════════════════════════════════════════════

async fn cmd_agent(action: AgentCommands) -> Result<()> {
    use bmux::agents::{commands as agent_cmd, AgentRegistry};

    let mut registry = AgentRegistry::new();

    match action {
        AgentCommands::Spawn { agent_type, name, model } => {
            agent_cmd::handle_spawn(&mut registry, &agent_type, &name, model.as_deref()).await
        }
        AgentCommands::List => {
            agent_cmd::handle_list(&registry);
            Ok(())
        }
        AgentCommands::Status { name } => {
            agent_cmd::handle_status(&registry, &name)
        }
        AgentCommands::Kill { name } => {
            agent_cmd::handle_kill(&mut registry, &name).await
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Task commands (Epic 3)
// ═══════════════════════════════════════════════════════════════════════════════

async fn cmd_task(action: TaskCommands) -> Result<()> {
    use bmux::orchestration::message_bus::MessageBus;
    use bmux::orchestration::task_router::TaskRouter;
    use std::sync::Arc;

    // Connect to the running session's message bus
    // For now, create a local instance (full integration needs session IPC)
    let bus = Arc::new(MessageBus::new("default")?);
    let router = TaskRouter::new(bus, 300);

    match action {
        TaskCommands::Send { agent, content, auto: _, model } => {
            if let Some(agent_name) = agent {
                let result = router.send_to_agent(&agent_name, &content, 1).await?;
                println!("{:?}", result);
            } else {
                let result = router.send_auto(&content, model.as_deref(), None).await?;
                println!("{:?}", result);
            }
            Ok(())
        }
        TaskCommands::List => {
            let tasks = router.list_tasks().await;
            if tasks.is_empty() {
                println!("No tasks.");
            } else {
                for t in &tasks {
                    println!("{:<8} {:<15} {:<10}", t.id, t.destination, t.status);
                }
            }
            Ok(())
        }
        TaskCommands::Cancel { id } => {
            router.cancel_task(&id).await?;
            println!("Task {} cancelled.", id);
            Ok(())
        }
        TaskCommands::Status { id } => {
            let task = router.task_status(&id).await
                .ok_or_else(|| anyhow::anyhow!("Task '{}' not found", id))?;
            println!("{:#?}", task);
            Ok(())
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Context commands (Epic 4)
// ═══════════════════════════════════════════════════════════════════════════════

fn cmd_context(action: ContextCommands) -> Result<()> {
    use bmux::storage::context_store::ContextStore;

    let db_path = dirs::data_local_dir()
        .unwrap_or_default()
        .join("bmux")
        .join("context");
    let store = ContextStore::open("default", &db_path)?;

    match action {
        ContextCommands::Set { key, value } => {
            store.set(&key, &value, None)?;
            println!("Set '{}'.", key);
            Ok(())
        }
        ContextCommands::Get { key } => {
            match store.get(&key)? {
                Some(val) => println!("{}", val),
                None => println!("Key not found: {}", key),
            }
            Ok(())
        }
        ContextCommands::List => {
            let entries = store.list()?;
            if entries.is_empty() {
                println!("No context entries.");
            } else {
                for (key, preview) in &entries {
                    println!("{:<40} {}", key, preview);
                }
            }
            Ok(())
        }
        ContextCommands::Dump => {
            let json = store.dump_json()?;
            println!("{}", json);
            Ok(())
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Workflow commands (Epic 5)
// ═══════════════════════════════════════════════════════════════════════════════

async fn cmd_workflow(action: WorkflowCommands) -> Result<()> {
    use bmux::config::settings::BmuxConfig;
    use bmux::workflow::yaml_parser::WorkflowParser;

    let parser = WorkflowParser::new();
    let settings = BmuxConfig::load().unwrap_or_default();

    match action {
        WorkflowCommands::Run { file, input: _, dry_run } => {
            let dag = parser.parse_file(&file)?;

            if dry_run {
                let summary = parser.dry_run_summary(&dag, &settings);
                println!("{}", summary);
            } else {
                // Full execution requires a running session with context store
                println!("Workflow execution requires an active bmux session.");
                println!("Use --dry-run to validate, or run from within a session.");
            }
            Ok(())
        }
        WorkflowCommands::List => {
            let workflow_dir = dirs::config_dir()
                .unwrap_or_default()
                .join("bmux")
                .join("workflows");
            if !workflow_dir.exists() {
                println!("No workflows directory found at {}", workflow_dir.display());
                return Ok(());
            }
            let summaries = parser.list_workflows(workflow_dir.to_str().unwrap_or("."))?;
            if summaries.is_empty() {
                println!("No workflows found.");
            } else {
                for s in &summaries {
                    println!("{}", s);
                }
            }
            Ok(())
        }
        WorkflowCommands::Status { id } => {
            println!("Workflow status for '{}' — requires active session.", id);
            Ok(())
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Config commands (Story 1.8)
// ═══════════════════════════════════════════════════════════════════════════════

fn cmd_config(action: ConfigCommands) -> Result<()> {
    use bmux::config;

    match action {
        ConfigCommands::Show => {
            config::show_config();
            Ok(())
        }
        ConfigCommands::Validate => {
            config::validate_config();
            Ok(())
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Data commands (Story 6.3)
// ═══════════════════════════════════════════════════════════════════════════════

fn cmd_data(action: DataCommands) -> Result<()> {
    

    match action {
        DataCommands::Purge => {
            // Purge audit logs
            let audit_dir = dirs::data_local_dir()
                .unwrap_or_default()
                .join("bmux")
                .join("audit");
            if audit_dir.exists() {
                std::fs::remove_dir_all(&audit_dir)?;
                println!("Audit logs purged.");
            }

            // Purge context store
            let ctx_dir = dirs::data_local_dir()
                .unwrap_or_default()
                .join("bmux")
                .join("context");
            if ctx_dir.exists() {
                std::fs::remove_dir_all(&ctx_dir)?;
                println!("Context data purged.");
            }

            // Purge session metadata
            let session_dir = std::path::PathBuf::from("/tmp/bmux");
            if session_dir.exists() {
                std::fs::remove_dir_all(&session_dir)?;
                println!("Session data purged.");
            }

            println!("All BMUX local data purged.");
            Ok(())
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(unix)]
fn send_sigterm(pid: u32) {
    let _ = std::process::Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status();
}

#[cfg(not(unix))]
fn send_sigterm(_pid: u32) {
    eprintln!("SIGTERM not supported on this platform");
}

async fn run_daemon(name: &str) -> Result<()> {
    use bmux::tui::server::DaemonServer;
    let server = DaemonServer::new(name, 24, 80)?;
    server.run().await
}
