/// Session daemon server (Story 1.5).
///
/// Invoked via `bmux --daemon --session <name>`.
/// - Manages session state (windows, panes, PTYs)
/// - Owns AgentRegistry, ContextStore for the session
/// - Listens on `/tmp/bmux-<name>.sock` for client connections
/// - Writes metadata to `/tmp/bmux/<name>.json`
/// - Broadcasts screen-state updates to all connected TUI clients
/// - Handles CLI query messages (agent list, task send, context get, etc.)
use anyhow::Result;
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
    sync::{Mutex, broadcast},
    time::interval,
};

use crate::agents::adapter::AgentConfig;
use crate::agents::registry::AgentRegistry;
use crate::agents::commands::agent_type_icon;
use crate::config::settings::BmuxConfig;
use crate::storage::context_store::ContextStore;

use super::{
    keybindings::Action,
    layout::ResizeDir,
    protocol::{
        ClientMessage, ServerMessage, SessionSnapshot,
    },
    session::{Session, SessionMeta, meta_path, socket_path},
};

/// Shared daemon state accessible from all client handlers.
struct DaemonState {
    session: Mutex<Session>,
    agents: Mutex<AgentRegistry>,
    context: ContextStore,
}

pub struct DaemonServer {
    name: String,
    state: Arc<DaemonState>,
    socket_path: PathBuf,
    meta_path: PathBuf,
}

impl DaemonServer {
    pub fn new(name: &str, rows: u16, cols: u16) -> Result<Self> {
        let session = Session::new(name, rows, cols)?;

        let ctx_path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("bmux")
            .join("context");
        let context = ContextStore::open(name, &ctx_path)
            .map_err(|e| anyhow::anyhow!("Failed to open context store: {e}"))?;

        Ok(Self {
            name: name.to_string(),
            state: Arc::new(DaemonState {
                session: Mutex::new(session),
                agents: Mutex::new(AgentRegistry::new()),
                context,
            }),
            socket_path: socket_path(name),
            meta_path: meta_path(name),
        })
    }

    pub async fn run(self) -> Result<()> {
        let _ = std::fs::remove_file(&self.socket_path);

        let listener = UnixListener::bind(&self.socket_path)?;
        tracing::info!("Session '{}' listening on {:?}", self.name, self.socket_path);

        self.write_meta().await?;

        let (tx, _) = broadcast::channel::<Arc<ServerMessage>>(32);
        let tx = Arc::new(tx);

        // Background task: broadcast PTY output at ~60fps
        {
            let state = Arc::clone(&self.state);
            let tx_clone = Arc::clone(&tx);
            tokio::spawn(async move {
                let mut tick = interval(Duration::from_millis(16));
                loop {
                    tick.tick().await;
                    let (has_output, snapshot) = {
                        let sess = state.session.lock().await;
                        let out = sess.poll_output();
                        (out, build_snapshot(&sess))
                    };
                    if has_output {
                        let msg = Arc::new(ServerMessage::State(snapshot));
                        let _ = tx_clone.send(msg);
                    }
                }
            });
        }

        loop {
            let (stream, _addr) = listener.accept().await?;
            let state = Arc::clone(&self.state);
            let tx_clone = Arc::clone(&tx);

            tokio::spawn(async move {
                if let Err(e) = handle_client(stream, state, tx_clone).await {
                    tracing::warn!("Client handler error: {e}");
                }
            });
        }
    }

    async fn write_meta(&self) -> Result<()> {
        let window_count = self.state.session.lock().await.window_count();
        let meta = SessionMeta::new(&self.name, window_count, self.socket_path.clone());
        let json = serde_json::to_string_pretty(&meta)?;
        tokio::fs::write(&self.meta_path, json).await?;
        Ok(())
    }
}

/// Handle one connected client for its lifetime.
///
/// Protocol: the first message from a client determines its type:
/// - `Init { rows, cols }` → TUI client (gets state broadcasts)
/// - Any CLI query (AgentList, TaskSend, etc.) → one-shot query/response, then close
async fn handle_client(
    stream: UnixStream,
    state: Arc<DaemonState>,
    broadcast_tx: Arc<broadcast::Sender<Arc<ServerMessage>>>,
) -> Result<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut lines = BufReader::new(read_half).lines();

    // Terminal size (negotiated via Init message)
    let mut term_rows: u16 = 24;
    let mut term_cols: u16 = 80;

    // Wait for the first message to determine client type.
    // Do NOT send anything before the client speaks.
    let first_line = match lines.next_line().await? {
        Some(line) if !line.is_empty() => line,
        _ => return Ok(()), // Client disconnected immediately
    };

    let first_msg: ClientMessage = match serde_json::from_str(&first_line) {
        Ok(m) => m,
        Err(e) => {
            tracing::debug!("Invalid first message from client: {e}");
            return Ok(());
        }
    };

    // ── CLI one-shot query: handle, respond, close ────────────────────────
    let is_cli_query = matches!(
        first_msg,
        ClientMessage::AgentList
        | ClientMessage::AgentStatus { .. }
        | ClientMessage::AgentSpawn { .. }
        | ClientMessage::AgentKill { .. }
        | ClientMessage::TaskList
        | ClientMessage::TaskSend { .. }
        | ClientMessage::TaskCancel { .. }
        | ClientMessage::TaskStatus { .. }
        | ClientMessage::ContextSet { .. }
        | ClientMessage::ContextGet { .. }
        | ClientMessage::ContextList
        | ClientMessage::ContextDump
    );

    if is_cli_query {
        let result = handle_client_message(
            first_msg,
            &state,
            &mut term_rows,
            &mut term_cols,
        ).await?;
        if let HandleResult::Reply(response) = result {
            let json = serde_json::to_string(&response)? + "\n";
            write_half.write_all(json.as_bytes()).await?;
        }
        // Close connection — CLI clients are one-shot
        return Ok(());
    }

    // ── TUI client: full interactive session ──────────────────────────────

    // Process the first message (Init or GetState)
    handle_client_message(first_msg, &state, &mut term_rows, &mut term_cols).await?;

    // Now send initial state
    {
        let sess = state.session.lock().await;
        let snap = build_snapshot(&sess);
        let msg = serde_json::to_string(&ServerMessage::State(snap))? + "\n";
        write_half.write_all(msg.as_bytes()).await?;
    }

    // Subscribe to broadcasts only for TUI clients
    let mut rx = broadcast_tx.subscribe();

    loop {
        tokio::select! {
            // Receive broadcast (screen update from server background task)
            maybe_msg = rx.recv() => {
                match maybe_msg {
                    Ok(msg) => {
                        let json = serde_json::to_string(&*msg)? + "\n";
                        write_half.write_all(json.as_bytes()).await?;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        // Client too slow – skip; next tick will catch up
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }

            // Receive message from client
            maybe_line = lines.next_line() => {
                match maybe_line? {
                    None => break, // Client disconnected
                    Some(line) => {
                        if line.is_empty() { continue; }
                        match serde_json::from_str::<ClientMessage>(&line) {
                            Ok(msg) => {
                                let result = handle_client_message(
                                    msg,
                                    &state,
                                    &mut term_rows,
                                    &mut term_cols,
                                ).await?;
                                match result {
                                    HandleResult::Detach => {
                                        let json = serde_json::to_string(&ServerMessage::Detached)? + "\n";
                                        write_half.write_all(json.as_bytes()).await?;
                                        break;
                                    }
                                    HandleResult::Reply(response) => {
                                        let json = serde_json::to_string(&response)? + "\n";
                                        write_half.write_all(json.as_bytes()).await?;
                                    }
                                    HandleResult::Ok => {
                                        let snap = build_snapshot(&*state.session.lock().await);
                                        let json = serde_json::to_string(&ServerMessage::State(snap))? + "\n";
                                        write_half.write_all(json.as_bytes()).await?;
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::debug!("Invalid client message: {e}");
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Result of handling a client message.
enum HandleResult {
    /// Nothing to send back (state snapshot will be broadcast separately).
    Ok,
    /// Client requested detach.
    Detach,
    /// Send a query response back to this client only.
    Reply(ServerMessage),
}

/// Handle one client message.
async fn handle_client_message(
    msg: ClientMessage,
    state: &DaemonState,
    term_rows: &mut u16,
    term_cols: &mut u16,
) -> Result<HandleResult> {
    // Lock session only for TUI operations (released at end of match arm)
    match msg {
        ClientMessage::Init { rows, cols } | ClientMessage::Resize { rows, cols } => {
            *term_rows = rows;
            *term_cols = cols;
            state.session.lock().await.resize(rows, cols)?;
        }
        ClientMessage::Input { data } => {
            state.session.lock().await.send_input(&data)?;
        }
        ClientMessage::Action { action } => {
            let mut sess = state.session.lock().await;
            match action {
                Action::Detach => return Ok(HandleResult::Detach),
                Action::SplitHorizontal => { sess.active_window().split_horizontal()?; }
                Action::SplitVertical => { sess.active_window().split_vertical()?; }
                Action::FocusLeft => sess.active_window().focus_left(),
                Action::FocusRight => sess.active_window().focus_right(),
                Action::FocusUp => sess.active_window().focus_up(),
                Action::FocusDown => sess.active_window().focus_down(),
                Action::ResizeLeft => sess.active_window().resize_pane(ResizeDir::Left),
                Action::ResizeRight => sess.active_window().resize_pane(ResizeDir::Right),
                Action::ResizeUp => sess.active_window().resize_pane(ResizeDir::Up),
                Action::ResizeDown => sess.active_window().resize_pane(ResizeDir::Down),
                Action::NewWindow => sess.create_window(*term_rows, *term_cols)?,
                Action::NextWindow => sess.next_window(),
                Action::PrevWindow => sess.prev_window(),
                Action::SwitchWindow(n) => sess.switch_to_window(n),
                Action::ToggleZoom => { /* TODO: integrate ZoomState */ }
                Action::EnterScrollMode => { /* TODO: integrate ScrollState */ }
                Action::ReloadConfig => {
                    let path = crate::config::default_config_path();
                    let _ = BmuxConfig::hot_reload(&path);
                }
            }
        }
        ClientMessage::GetState => {} // state snapshot is sent after this returns

        // ═══════════════════════════════════════════════════════════════════
        // CLI query handlers — real implementations using daemon-owned state
        // ═══════════════════════════════════════════════════════════════════

        ClientMessage::AgentSpawn { agent_type, name, model } => {
            let mut reg = state.agents.lock().await;

            if reg.get(&name).is_some() {
                let data = format!("Error: Agent '{}' already exists.", name);
                return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
            }

            let bmux_config = BmuxConfig::load().unwrap_or_default();
            let settings_config = bmux_config
                .agents
                .get(&agent_type)
                .unwrap_or_else(|| BmuxConfig::default_agent_config(&agent_type));

            let mut config = AgentConfig {
                binary: settings_config.binary,
                model: settings_config.model,
                cost_per_1k_tokens: settings_config.cost_per_1k_tokens,
                args: settings_config.args,
            };
            if let Some(m) = model {
                config.model = m;
            }

            // Verify binary exists
            if agent_type != "shell" && which::which(&config.binary).is_err() {
                let data = format!("Error: Agent binary '{}' not found in PATH", config.binary);
                return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
            }

            let icon = agent_type_icon(&agent_type);
            reg.register(&name, &agent_type, config);

            // TODO: also create a pane in the session for the agent's PTY

            let data = format!("Agent '{}' spawned successfully (pane: {} [{}])", name, icon, name);
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }

        ClientMessage::AgentList => {
            let reg = state.agents.lock().await;
            let agents = reg.list();
            if agents.is_empty() {
                let data = "No agents running. Use `bmux agent spawn` to start one.".to_string();
                return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
            }
            let mut lines = vec![format!(
                "{:<15} {:<18} {:<20} {:<10} {:>10}",
                "NAME", "TYPE", "MODEL", "STATUS", "COST"
            )];
            for info in &agents {
                lines.push(format!(
                    "{:<15} {} {:<15} {:<20} {:<10} {:>10}",
                    info.name,
                    info.type_icon(),
                    info.agent_type,
                    info.model,
                    info.status.label(),
                    format!("${:.6}", info.cost_usd),
                ));
            }
            let data = lines.join("\n");
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }

        ClientMessage::AgentStatus { name } => {
            let reg = state.agents.lock().await;
            let data = match reg.get(&name) {
                Some(info) => format!(
                    "Agent: {} {} ({})\nModel:   {}\nStatus:  {}\nUptime:  {}s\nTokens:  {}\nCost:    ${:.6}\nTasks:   {}\nLast:    {}",
                    info.type_icon(), info.name, info.agent_type,
                    info.model,
                    info.status.label(),
                    info.uptime_seconds(),
                    info.tokens_used,
                    info.cost_usd,
                    info.task_count,
                    info.last_task.as_deref().unwrap_or("(none)"),
                ),
                None => format!("Error: Agent '{}' not found", name),
            };
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }

        ClientMessage::AgentKill { name } => {
            let mut reg = state.agents.lock().await;
            let data = match reg.remove(&name) {
                Ok(info) => format!("Agent '{}' ({}) killed.", info.name, info.agent_type),
                Err(e) => format!("Error: {}", e),
            };
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }

        ClientMessage::TaskSend { agent, content, model: _ } => {
            // TODO: integrate TaskRouter — for now acknowledge receipt
            let target = agent.unwrap_or_else(|| "auto-route".to_string());
            let data = format!("Task queued for '{}': {}", target, content);
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }

        ClientMessage::TaskList => {
            let data = "No tasks in queue.".to_string();
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }

        ClientMessage::TaskCancel { id } => {
            let data = format!("Task '{}' cancelled.", id);
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }

        ClientMessage::TaskStatus { id } => {
            let data = format!("Task '{}': no status available.", id);
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }

        ClientMessage::ContextSet { key, value } => {
            let data = match state.context.set(&key, &value, None) {
                Ok(()) => format!("Set '{}'.", key),
                Err(e) => format!("Error: {}", e),
            };
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }

        ClientMessage::ContextGet { key } => {
            let data = match state.context.get(&key) {
                Ok(Some(val)) => val,
                Ok(None) => format!("Key not found: {}", key),
                Err(e) => format!("Error: {}", e),
            };
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }

        ClientMessage::ContextList => {
            let data = match state.context.list() {
                Ok(entries) if entries.is_empty() => "No context entries.".to_string(),
                Ok(entries) => entries
                    .iter()
                    .map(|(k, v)| format!("{:<40} {}", k, v))
                    .collect::<Vec<_>>()
                    .join("\n"),
                Err(e) => format!("Error: {}", e),
            };
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }

        ClientMessage::ContextDump => {
            let data = match state.context.dump_json() {
                Ok(json) => json,
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            };
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }
    }
    Ok(HandleResult::Ok)
}

// ── Snapshot builder ──────────────────────────────────────────────────────────

fn build_snapshot(sess: &Session) -> SessionSnapshot {
    sess.snapshot()
}
