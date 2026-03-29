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
        /// Session name (auto-detects if only one is running)
        #[arg(short = 's', long, global = true)]
        session: Option<String>,
        #[command(subcommand)]
        action: AgentCommands,
    },

    // ── Task routing (Epic 3) ──────────────────────────────────────────────
    /// Send and manage tasks for agents
    Task {
        /// Session name
        #[arg(short = 's', long, global = true)]
        session: Option<String>,
        #[command(subcommand)]
        action: TaskCommands,
    },

    // ── Shared context (Epic 4) ────────────────────────────────────────────
    /// Read/write shared context store
    Context {
        /// Session name
        #[arg(short = 's', long, global = true)]
        session: Option<String>,
        #[command(subcommand)]
        action: ContextCommands,
    },

    // ── Workflow orchestration (Epic 5) ────────────────────────────────────
    /// Run and manage multi-agent workflows
    Workflow {
        /// Session name
        #[arg(short = 's', long, global = true)]
        session: Option<String>,
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
        Some(Commands::Agent { session, action }) => cmd_agent(session.as_deref(), action).await,
        Some(Commands::Task { session, action }) => cmd_task(session.as_deref(), action).await,
        Some(Commands::Context { session, action }) => cmd_context(session.as_deref(), action).await,
        Some(Commands::Workflow { session, action }) => cmd_workflow(session.as_deref(), action).await,
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
// Agent commands (Epic 2) — all queries go through session daemon IPC
// ═══════════════════════════════════════════════════════════════════════════════

async fn cmd_agent(session: Option<&str>, action: AgentCommands) -> Result<()> {
    use bmux::tui::ipc_client::{query, resolve_session};
    use bmux::tui::protocol::ClientMessage;

    let sock = resolve_session(session)?;

    let msg = match action {
        AgentCommands::Spawn { agent_type, name, model } => {
            ClientMessage::AgentSpawn { agent_type, name, model }
        }
        AgentCommands::List => ClientMessage::AgentList,
        AgentCommands::Status { name } => ClientMessage::AgentStatus { name },
        AgentCommands::Kill { name } => ClientMessage::AgentKill { name },
    };

    let result = query(&sock, msg).await?;
    println!("{}", result);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Task commands (Epic 3) — all queries go through session daemon IPC
// ═══════════════════════════════════════════════════════════════════════════════

async fn cmd_task(session: Option<&str>, action: TaskCommands) -> Result<()> {
    use bmux::tui::ipc_client::{query, resolve_session};
    use bmux::tui::protocol::ClientMessage;

    let sock = resolve_session(session)?;

    let msg = match action {
        TaskCommands::Send { agent, content, auto: _, model } => {
            ClientMessage::TaskSend { agent, content, model }
        }
        TaskCommands::List => ClientMessage::TaskList,
        TaskCommands::Cancel { id } => ClientMessage::TaskCancel { id },
        TaskCommands::Status { id } => ClientMessage::TaskStatus { id },
    };

    let result = query(&sock, msg).await?;
    println!("{}", result);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Context commands (Epic 4) — all queries go through session daemon IPC
// ═══════════════════════════════════════════════════════════════════════════════

async fn cmd_context(session: Option<&str>, action: ContextCommands) -> Result<()> {
    use bmux::tui::ipc_client::{query, resolve_session};
    use bmux::tui::protocol::ClientMessage;

    let sock = resolve_session(session)?;

    let msg = match action {
        ContextCommands::Set { key, value } => ClientMessage::ContextSet { key, value },
        ContextCommands::Get { key } => ClientMessage::ContextGet { key },
        ContextCommands::List => ClientMessage::ContextList,
        ContextCommands::Dump => ClientMessage::ContextDump,
    };

    let result = query(&sock, msg).await?;
    println!("{}", result);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Workflow commands (Epic 5) — dry-run is local, execution goes through IPC
// ═══════════════════════════════════════════════════════════════════════════════

async fn cmd_workflow(session: Option<&str>, action: WorkflowCommands) -> Result<()> {
    use bmux::config::settings::BmuxConfig;
    use bmux::workflow::yaml_parser::WorkflowParser;

    match action {
        WorkflowCommands::Run { file, input: _, dry_run } => {
            let parser = WorkflowParser::new();
            let settings = BmuxConfig::load().unwrap_or_default();
            let dag = parser.parse_file(&file)?;

            if dry_run {
                let summary = parser.dry_run_summary(&dag, &settings);
                println!("{}", summary);
            } else {
                // TODO: send WorkflowRun message to daemon via IPC
                let _sock = bmux::tui::ipc_client::resolve_session(session)?;
                println!("Workflow execution via IPC — not yet implemented.");
                println!("Use --dry-run to validate.");
            }
            Ok(())
        }
        WorkflowCommands::List => {
            let parser = WorkflowParser::new();
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
            // TODO: query daemon for workflow status
            let _sock = bmux::tui::ipc_client::resolve_session(session)?;
            println!("Workflow status for '{}' — not yet implemented via IPC.", id);
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
