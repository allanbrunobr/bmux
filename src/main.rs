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
    /// Create a new session and attach to it
    New {
        /// Session name (default: the first free number, as in tmux)
        name: Option<String>,
    },
    /// Attach to an existing session (the only one, when NAME is omitted)
    Attach {
        /// Session name
        name: Option<String>,
        /// Session name (same as NAME)
        #[arg(short = 's', conflicts_with = "name")]
        session: Option<String>,
    },
    /// List all active sessions
    Ls,
    /// Kill a session and all its processes
    Kill {
        /// Session name
        name: Option<String>,
        /// Session name (same as NAME)
        #[arg(short = 's', conflicts_with = "name")]
        session: Option<String>,
    },

    // ── Agent management ──────────────────────────────────────────
    /// Manage AI agents (spawn, list, status, kill)
    Agent {
        /// Session name (auto-detects if only one is running)
        #[arg(short = 's', long, global = true)]
        session: Option<String>,
        #[command(subcommand)]
        action: AgentCommands,
    },

    // ── Task routing ──────────────────────────────────────────────
    /// Send and manage tasks for agents
    Task {
        /// Session name
        #[arg(short = 's', long, global = true)]
        session: Option<String>,
        #[command(subcommand)]
        action: TaskCommands,
    },

    // ── Shared context ────────────────────────────────────────────
    /// Read/write shared context store
    Context {
        /// Session name
        #[arg(short = 's', long, global = true)]
        session: Option<String>,
        #[command(subcommand)]
        action: ContextCommands,
    },

    // ── Workflow orchestration ────────────────────────────────────
    /// Run and manage multi-agent workflows
    Workflow {
        /// Session name
        #[arg(short = 's', long, global = true)]
        session: Option<String>,
        #[command(subcommand)]
        action: WorkflowCommands,
    },

    // ── Configuration ──────────────────────────────────────────
    /// Show, validate or reload configuration
    Config {
        /// Session name for `config reload`
        #[arg(short = 's', long, global = true)]
        session: Option<String>,
        #[command(subcommand)]
        action: ConfigCommands,
    },

    // ── Data management ────────────────────────────────────────
    /// Manage local BMUX data
    Data {
        #[command(subcommand)]
        action: DataCommands,
    },
}

// ── Agent subcommands ─────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
enum AgentCommands {
    /// Spawn an AI agent in a new pane
    Spawn {
        /// Agent type: claude-code (alias: claude), opencode, pi, gemini, shell,
        /// or a custom [agents.<name>] entry. Default: `[agents] default_type`.
        #[arg(value_name = "TYPE")]
        type_arg: Option<String>,
        /// Agent type (same as TYPE)
        #[arg(long = "type", conflicts_with = "type_arg")]
        agent_type: Option<String>,
        /// Agent name
        #[arg(long)]
        name: String,
        /// Model override
        #[arg(long)]
        model: Option<String>,
    },
    /// List all active agents
    #[command(visible_alias = "ls")]
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
    /// Send a task to an agent: `task send dev "write tests"`, or
    /// `task send --auto "write tests"` to let the router pick the agent
    Send {
        /// Agent name followed by the task text. With --auto, only the task text.
        #[arg(required = true, num_args = 1.., value_name = "AGENT> <TEXT",
              trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
        /// Auto-route to best available agent
        #[arg(long)]
        auto: bool,
        /// Prefer a specific model
        #[arg(long)]
        model: Option<String>,
    },
    /// List all tasks
    #[command(visible_alias = "ls")]
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
    #[command(visible_alias = "ls")]
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
    #[command(visible_alias = "ls")]
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
    /// Reload config.toml in a running session (same as `Ctrl-b r`)
    Reload,
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
        Some(Commands::New { name }) => {
            let name = name.unwrap_or_else(next_session_name);
            cmd_new(&name).await
        }
        Some(Commands::Attach { name, session }) => cmd_attach(name.or(session).as_deref()).await,
        Some(Commands::Ls) => cmd_ls(),
        Some(Commands::Kill { name, session }) => match name.or(session) {
            Some(name) => cmd_kill(&name),
            None => anyhow::bail!("Specify the session to kill: `bmux kill <name>`."),
        },
        Some(Commands::Agent { session, action }) => cmd_agent(session.as_deref(), action).await,
        Some(Commands::Task { session, action }) => cmd_task(session.as_deref(), action).await,
        Some(Commands::Context { session, action }) => {
            cmd_context(session.as_deref(), action).await
        }
        Some(Commands::Workflow { session, action }) => {
            cmd_workflow(session.as_deref(), action).await
        }
        Some(Commands::Config { session, action }) => cmd_config(session.as_deref(), action).await,
        Some(Commands::Data { action }) => cmd_data(action),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Session commands
// ═══════════════════════════════════════════════════════════════════════════════

async fn cmd_new(name: &str) -> Result<()> {
    use bmux::tui::session::{list_sessions, socket_path};

    let existing = list_sessions();
    if existing.iter().any(|s| s.name == name) {
        eprintln!(
            "Session '{}' is already running. Use `bmux attach -s {}` to attach.",
            name, name
        );
        std::process::exit(1);
    }

    let exe = std::env::current_exe()?;
    let mut daemon = std::process::Command::new(&exe);
    daemon
        .args(["--daemon", "--session-name", name])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    // Start the daemon in its own session so it survives when the terminal
    // that created it is closed (it would get SIGHUP otherwise).
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: setsid is async-signal-safe and runs in the child before exec.
        unsafe {
            daemon.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
    let _child = daemon.spawn()?;

    let sock = socket_path(name);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        if sock.exists() {
            break;
        }
        if std::time::Instant::now() >= deadline {
            anyhow::bail!("Timed out waiting for session '{}' to start", name);
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    bmux::tui::client::run_client(sock).await
}

async fn cmd_attach(name: Option<&str>) -> Result<()> {
    // Same lookup as the other commands: name, then $BMUX_SESSION, then the
    // only running session.
    let sock = bmux::tui::ipc_client::resolve_session(name)?;
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
            println!(
                "{:<20} {:>8}  {}",
                s.name,
                s.window_count,
                s.created_at.format("%Y-%m-%d %H:%M:%S UTC")
            );
        }
    }
    Ok(())
}

fn cmd_kill(name: &str) -> Result<()> {
    use bmux::tui::session::{list_sessions, meta_path, socket_path};
    let sessions = list_sessions();
    let meta = sessions
        .iter()
        .find(|s| s.name == name)
        .ok_or_else(|| anyhow::anyhow!("No session named '{}'", name))?;
    send_sigterm(meta.pid);
    let _ = std::fs::remove_file(socket_path(name));
    let _ = std::fs::remove_file(meta_path(name));
    println!("Session '{}' killed.", name);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Agent commands — all queries go through session daemon IPC
// ═══════════════════════════════════════════════════════════════════════════════

async fn cmd_agent(session: Option<&str>, action: AgentCommands) -> Result<()> {
    use bmux::tui::ipc_client::{query, resolve_session};
    use bmux::tui::protocol::ClientMessage;

    let sock = resolve_session(session)?;

    let msg = match action {
        AgentCommands::Spawn {
            type_arg,
            agent_type,
            name,
            model,
        } => {
            let agent_type = match type_arg.or(agent_type) {
                Some(t) => t,
                None => bmux::config::settings::BmuxConfig::load()
                    .unwrap_or_default()
                    .default_agent_type()
                    .to_string(),
            };
            ClientMessage::AgentSpawn {
                agent_type: normalize_agent_type(&agent_type),
                name,
                model,
            }
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
// Task commands — all queries go through session daemon IPC
// ═══════════════════════════════════════════════════════════════════════════════

async fn cmd_task(session: Option<&str>, action: TaskCommands) -> Result<()> {
    use bmux::tui::ipc_client::{query, resolve_session};
    use bmux::tui::protocol::ClientMessage;

    let sock = resolve_session(session)?;

    let msg = match action {
        TaskCommands::Send { args, auto, model } => {
            let (agent, content) = split_task_args(args, auto)?;
            ClientMessage::TaskSend {
                agent,
                content,
                model,
            }
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
// Context commands — all queries go through session daemon IPC
// ═══════════════════════════════════════════════════════════════════════════════

async fn cmd_context(session: Option<&str>, action: ContextCommands) -> Result<()> {
    use bmux::tui::ipc_client::{query, resolve_session};
    use bmux::tui::protocol::ClientMessage;

    let sock = resolve_session(session)?;

    let msg = match action {
        ContextCommands::Set { key, value } => ClientMessage::ContextSet {
            key,
            value,
            author: None,
        },
        ContextCommands::Get { key } => ClientMessage::ContextGet { key },
        ContextCommands::List => ClientMessage::ContextList,
        ContextCommands::Dump => ClientMessage::ContextDump,
    };

    let result = query(&sock, msg).await?;
    println!("{}", result);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Workflow commands — dry-run is local, execution goes through IPC
// ═══════════════════════════════════════════════════════════════════════════════

async fn cmd_workflow(session: Option<&str>, action: WorkflowCommands) -> Result<()> {
    use bmux::config::settings::BmuxConfig;
    use bmux::workflow::yaml_parser::WorkflowParser;

    match action {
        WorkflowCommands::Run {
            file,
            input,
            dry_run,
        } => {
            let parser = WorkflowParser::new();
            let settings = BmuxConfig::load().unwrap_or_default();
            let dag = parser.parse_file(&file)?;
            // The daemon has its own working directory: send an absolute path.
            let file = std::fs::canonicalize(&file)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or(file);

            if dry_run {
                let summary = parser.dry_run_summary(&dag, &settings);
                println!("{}", summary);
            } else {
                use bmux::tui::ipc_client::{query, resolve_session};
                use bmux::tui::protocol::ClientMessage;
                let sock = resolve_session(session)?;
                let result = query(
                    &sock,
                    ClientMessage::WorkflowRun {
                        file: file.clone(),
                        input: input.clone(),
                    },
                )
                .await?;
                println!("{}", result);
            }
            Ok(())
        }
        WorkflowCommands::List => {
            let parser = WorkflowParser::new();
            // ~/.config/bmux/workflows, next to config.toml
            let workflow_dir = bmux::config::default_config_path()
                .parent()
                .map(|d| d.join("workflows"))
                .unwrap_or_default();
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
            use bmux::tui::ipc_client::{query, resolve_session};
            use bmux::tui::protocol::ClientMessage;
            let sock = resolve_session(session)?;
            let result = query(&sock, ClientMessage::WorkflowStatus { id: id.clone() }).await?;
            println!("{}", result);
            Ok(())
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Config commands
// ═══════════════════════════════════════════════════════════════════════════════

async fn cmd_config(session: Option<&str>, action: ConfigCommands) -> Result<()> {
    use bmux::config;

    match action {
        ConfigCommands::Reload => {
            use bmux::tui::ipc_client::{query, resolve_session};
            use bmux::tui::protocol::ClientMessage;
            let sock = resolve_session(session)?;
            let result = query(&sock, ClientMessage::ConfigReload).await?;
            println!("{}", result);
            if result.starts_with("Error") {
                std::process::exit(1);
            }
            Ok(())
        }
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
// Data commands
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

/// First number not used by a running session: "0", "1", ...
fn next_session_name() -> String {
    let taken: Vec<String> = bmux::tui::session::list_sessions()
        .into_iter()
        .map(|s| s.name)
        .collect();
    (0..)
        .map(|n: u32| n.to_string())
        .find(|n| !taken.contains(n))
        .unwrap_or_else(|| "0".to_string())
}

/// Accept `claude` as a short name for `claude-code`.
fn normalize_agent_type(agent_type: &str) -> String {
    match agent_type {
        "claude" => "claude-code".to_string(),
        other => other.to_string(),
    }
}

/// Split `task send` arguments into (agent, text). Without `--auto` the first
/// argument is the agent; the remaining arguments are joined into the text.
/// A `--` separator before the text (old syntax) is accepted and dropped.
fn split_task_args(args: Vec<String>, auto: bool) -> Result<(Option<String>, String)> {
    fn text(words: Vec<String>) -> String {
        let skip = usize::from(words.first().is_some_and(|w| w == "--"));
        words[skip..].join(" ")
    }
    if auto {
        let content = text(args);
        if content.trim().is_empty() {
            anyhow::bail!("Task text is empty.");
        }
        return Ok((None, content));
    }
    let mut it = args.into_iter();
    let agent = it.next().unwrap_or_default();
    let content = text(it.collect());
    if content.trim().is_empty() {
        anyhow::bail!(
            "Missing task text. Usage: bmux task send <agent> \"<text>\" or bmux task send --auto \"<text>\""
        );
    }
    Ok((Some(agent), content))
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(std::iter::once("bmux").chain(args.iter().copied())).unwrap()
    }

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn task_send_accepts_text_with_and_without_separator() {
        for argv in [
            &["task", "send", "dev", "write tests"][..],
            &["task", "send", "dev", "--", "write tests"][..],
            &["task", "send", "dev", "write", "tests"][..],
        ] {
            let Some(Commands::Task {
                action: TaskCommands::Send { args, auto, .. },
                ..
            }) = parse(argv).command
            else {
                panic!("not a task send: {argv:?}");
            };
            assert_eq!(
                split_task_args(args, auto).unwrap(),
                (Some("dev".to_string()), "write tests".to_string())
            );
        }
    }

    #[test]
    fn task_send_auto_uses_all_args_as_text() {
        assert_eq!(
            split_task_args(strings(&["fix", "the", "bug"]), true).unwrap(),
            (None, "fix the bug".to_string())
        );
        assert!(split_task_args(strings(&["dev"]), false).is_err());
    }

    #[test]
    fn agent_spawn_accepts_positional_type_and_alias() {
        let Some(Commands::Agent {
            action: AgentCommands::Spawn { type_arg, .. },
            ..
        }) = parse(&["agent", "spawn", "claude", "--name", "dev"]).command
        else {
            panic!("not an agent spawn");
        };
        assert_eq!(normalize_agent_type(&type_arg.unwrap()), "claude-code");
        assert!(
            Cli::try_parse_from(["bmux", "agent", "spawn", "--type", "pi", "--name", "x"]).is_ok()
        );
        assert!(Cli::try_parse_from(["bmux", "agent", "spawn", "--name", "x"]).is_ok());
    }

    #[test]
    fn ls_aliases_and_positional_session_names() {
        assert!(Cli::try_parse_from(["bmux", "agent", "ls"]).is_ok());
        assert!(Cli::try_parse_from(["bmux", "task", "ls"]).is_ok());
        assert!(Cli::try_parse_from(["bmux", "attach", "demo"]).is_ok());
        assert!(Cli::try_parse_from(["bmux", "attach", "-s", "demo"]).is_ok());
        assert!(Cli::try_parse_from(["bmux", "attach"]).is_ok());
        assert!(Cli::try_parse_from(["bmux", "kill", "demo"]).is_ok());
        assert!(Cli::try_parse_from(["bmux", "config", "reload"]).is_ok());
        assert!(Cli::try_parse_from(["bmux", "new"]).is_ok());
    }
}
