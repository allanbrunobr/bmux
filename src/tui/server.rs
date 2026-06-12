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
use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
    sync::{Mutex, broadcast},
    time::interval,
};

use crate::agents::adapter::AgentConfig;
use crate::agents::registry::AgentRegistry;
use crate::agents::commands::agent_type_icon;
use crate::agents::runtime::AgentRuntime;
use crate::config::settings::BmuxConfig;
use crate::orchestration::message_bus::{MessageBus, MessageType, ResultPayload};
use crate::orchestration::task_router::{self, TaskRouter};
use crate::security::envelope::{harden_socket_permissions, SecurityEnvelope};
use crate::storage::context_store::ContextStore;
use crate::workflow::engine::WorkflowEngine;
use crate::workflow::yaml_parser::WorkflowParser;

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
    context: Arc<ContextStore>,
    task_router: Arc<TaskRouter>,
    workflow_engine: Arc<WorkflowEngine>,
    envelope: Arc<SecurityEnvelope>,
    bmux_config: BmuxConfig,
    session_name: String,
    ipc_socket_path: PathBuf,
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
        let context = Arc::new(
            ContextStore::open(name, &ctx_path)
                .map_err(|e| anyhow::anyhow!("Failed to open context store: {e}"))?,
        );

        let bmux_config = BmuxConfig::load().unwrap_or_default();
        let bus = Arc::new(MessageBus::new(name)?);
        let ipc_socket_path = bus.socket_path().clone();
        let task_router = Arc::new(TaskRouter::new(bus, 300));
        let workflow_engine = Arc::new(WorkflowEngine::with_shared(
            Arc::clone(&context),
            Arc::clone(&task_router),
        ));
        let envelope = Arc::new(SecurityEnvelope::new(
            name,
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            bmux_config.security.clone(),
            bmux_config.audit.enabled,
        ));

        Ok(Self {
            name: name.to_string(),
            state: Arc::new(DaemonState {
                session: Mutex::new(session),
                agents: Mutex::new(AgentRegistry::new()),
                context,
                task_router,
                workflow_engine,
                envelope,
                bmux_config,
                session_name: name.to_string(),
                ipc_socket_path,
            }),
            socket_path: socket_path(name),
            meta_path: meta_path(name),
        })
    }

    pub async fn run(self) -> Result<()> {
        let _ = std::fs::remove_file(&self.socket_path);

        let listener = UnixListener::bind(&self.socket_path)?;
        harden_socket_permissions(&self.socket_path)?;
        tracing::info!("Session '{}' listening on {:?}", self.name, self.socket_path);

        self.state.task_router.message_bus().start().await?;
        spawn_result_subscriber(Arc::clone(&self.state.task_router));
        spawn_task_dispatch_subscriber(Arc::clone(&self.state));

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
        | ClientMessage::WorkflowRun { .. }
        | ClientMessage::WorkflowStatus { .. }
    );

    // Drag state for mouse border resize (per-client)
    let mut drag_left_pane: Option<usize> = None;

    if is_cli_query {
        let result = handle_client_message(
            first_msg,
            &state,
            &mut term_rows,
            &mut term_cols,
            &mut drag_left_pane,
        ).await?;
        if let HandleResult::Reply(response) = result {
            let json = serde_json::to_string(&response)? + "\n";
            write_half.write_all(json.as_bytes()).await?;
        }

        // CLI queries that mutate session state (AgentSpawn, AgentKill)
        // must broadcast updated snapshot to all TUI clients.
        let snap = build_snapshot(&*state.session.lock().await);
        let _ = broadcast_tx.send(Arc::new(ServerMessage::State(snap)));

        // Close connection — CLI clients are one-shot
        return Ok(());
    }

    // ── TUI client: full interactive session ──────────────────────────────

    // Process the first message (Init or GetState)
    handle_client_message(first_msg, &state, &mut term_rows, &mut term_cols, &mut drag_left_pane).await?;

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
                                    &mut drag_left_pane,
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
    drag_left_pane: &mut Option<usize>,
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
                Action::FocusNextPane => sess.active_window().focus_next_pane(),
                Action::ToggleZoom => { /* TODO: integrate ZoomState */ }
                Action::EnterScrollMode => { /* TODO: integrate ScrollState */ }
                Action::ReloadConfig => {
                    let path = crate::config::default_config_path();
                    let _ = BmuxConfig::hot_reload(&path);
                }
            }
        }
        ClientMessage::GetState => {} // state snapshot is sent after this returns
        ClientMessage::MouseClick { col, row } => {
            let mut sess = state.session.lock().await;
            // Check if clicking on a border (start drag)
            if let Some((left_id, _right_id, _border_x)) = sess.border_at(col, row) {
                *drag_left_pane = Some(left_id);
            } else {
                sess.focus_pane_at(col, row);
                *drag_left_pane = None;
            }
        }
        ClientMessage::MouseDrag { col, row } => {
            let mut sess = state.session.lock().await;
            // Update hover for border highlighting
            sess.active_window().set_hover(col, row);
            // Apply drag resize if active
            if let Some(left_id) = *drag_left_pane {
                sess.active_window().drag_border(left_id, col);
            }
        }
        ClientMessage::MouseUp => {
            *drag_left_pane = None;
        }

        // ═══════════════════════════════════════════════════════════════════
        // CLI query handlers — real implementations using daemon-owned state
        // ═══════════════════════════════════════════════════════════════════

        ClientMessage::AgentSpawn { agent_type, name, model } => {
            // Check for duplicate name (short lock)
            {
                let reg = state.agents.lock().await;
                if reg.get(&name).is_some() {
                    let data = format!("Error: Agent '{}' already exists.", name);
                    return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
                }
            } // agents lock dropped here

            let runtime = AgentRuntime::new(
                &state.bmux_config,
                &state.envelope,
                &state.ipc_socket_path,
            );

            let config = if agent_type == "custom" {
                match state.bmux_config.agents.get_custom(&name) {
                    Some(s) => AgentConfig {
                        binary: s.binary,
                        model: model.clone().unwrap_or(s.model),
                        cost_per_1k_tokens: s.cost_per_1k_tokens,
                        args: s.args,
                    },
                    None => {
                        let data = format!(
                            "Custom agent '{}' not found in config.toml. Add [agents.{}] section.",
                            name, name
                        );
                        return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
                    }
                }
            } else {
                runtime.resolve_config(&agent_type, model.as_deref())
            };

            if agent_type != "shell" && which::which(&config.binary).is_err() {
                let data = format!("Error: Agent binary '{}' not found in PATH", config.binary);
                return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
            }

            let agent_cmd = runtime.build_launch_command(&name, &agent_type, &config);

            // Split window — session lock acquired and released in this block
            let pane_id = {
                let mut sess = state.session.lock().await;
                match sess.split_and_run_command(&agent_cmd) {
                    Ok(id) => id,
                    Err(e) => {
                        let data = format!("Error creating agent pane: {}", e);
                        return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
                    }
                }
            }; // session lock dropped here

            let icon = agent_type_icon(&agent_type);
            let cost = config.cost_per_1k_tokens;
            let model_name = config.model.clone();

            // Register — agents lock acquired here (after session lock released)
            {
                let mut reg = state.agents.lock().await;
                reg.register(&name, &agent_type, config);
                if let Some(info) = reg.get_mut(&name) {
                    info.pane_id = Some(pane_id);
                }
            } // agents lock dropped here

            // Register in TaskRouter (no lock contention)
            state.task_router.register_agent(task_router::AgentInfo {
                id: name.clone(),
                agent_type: agent_type.clone(),
                model_name: model_name.clone(),
                cost_per_1k_tokens: cost,
                capabilities: vec![],
                idle_since: Some(std::time::Instant::now()),
            }).await;

            let _ = state.envelope.log_agent_spawned(&name, &agent_type, &model_name);

            let data = format!(
                "Agent '{}' spawned successfully (pane: {} [{}])\n\
                 Pane ID: {}\n\
                 Session: {}\n\
                 Env: BMUX_SESSION={} BMUX_SOCKET={}",
                name, icon, name,
                pane_id,
                state.session_name,
                state.session_name,
                state.ipc_socket_path.display(),
            );
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
                    "Agent: {} {} ({})\nPID:     {}\nModel:   {}\nStatus:  {}\nUptime:  {}s\nTokens:  {}\nCost:    ${:.6}\nTasks:   {}\nLast:    {}",
                    info.type_icon(), info.name, info.agent_type,
                    info.pid.map(|p| p.to_string()).unwrap_or_else(|| "(no process)".into()),
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
            let (data, pane_id, agent_type) = {
                let mut reg = state.agents.lock().await;
                match reg.remove(&name) {
                    Ok(info) => {
                        let pane_id = info.pane_id;
                        let agent_type = info.agent_type.clone();
                        (
                            format!("Agent '{}' ({}) killed.", info.name, info.agent_type),
                            pane_id,
                            agent_type,
                        )
                    }
                    Err(e) => (format!("Error: {}", e), None, String::new()),
                }
            };
            if let Some(pane_id) = pane_id {
                let mut sess = state.session.lock().await;
                if let Err(e) = sess.kill_pane(pane_id) {
                    tracing::warn!(agent = %name, pane = pane_id, "Failed to kill agent pane: {e}");
                }
            }
            state.task_router.handle_agent_crash(&name).await;
            if !agent_type.is_empty() {
                let _ = state.envelope.log_agent_killed(&name, &agent_type);
            }
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }

        ClientMessage::TaskSend { agent, content, model } => {
            // Determine the target agent name (direct or auto-routed)
            let (data, _target_agent, _task_id) = if let Some(ref agent_name) = agent {
                // Direct send to named agent
                match state.task_router.send_to_agent(agent_name, &content, 1).await {
                    Ok(task_router::SendResult::Dispatched { task_id }) => {
                        (format!("Task dispatched → {agent_name} (task_id: {task_id})"),
                         Some(agent_name.clone()),
                         Some(task_id))
                    }
                    Ok(task_router::SendResult::Queued { task_id, agent_id, position }) => {
                        (format!("Agent '{agent_id}' busy — task queued (position: {position}, task_id: {task_id})"),
                         None,
                         Some(task_id))
                    }
                    Err(e) => (format!("Error: {e}"), None, None),
                }
            } else {
                // Auto-route to best available agent
                match state.task_router.send_auto(&content, model.as_deref(), None).await {
                    Ok(task_router::AutoRouteResult::Dispatched { task_id, agent_id, estimated_cost }) => {
                        (format!("Task auto-routed → {agent_id} (cost: ${estimated_cost:.6}, task_id: {task_id})"),
                         Some(agent_id),
                         Some(task_id))
                    }
                    Ok(task_router::AutoRouteResult::AllBusy { task_id, timeout_seconds }) => {
                        (format!("All agents busy — queued (timeout: {timeout_seconds}s, task_id: {task_id})"),
                         None,
                         Some(task_id))
                    }
                    Err(e) => (format!("Error: {e}"), None, None),
                }
            };

            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }

        ClientMessage::TaskList => {
            let tasks = state.task_router.list_tasks().await;
            let data = if tasks.is_empty() {
                "No tasks.".to_string()
            } else {
                let mut lines = vec![format!(
                    "{:<10} {:<15} {:<12} {}",
                    "ID", "AGENT", "STATUS", "SUBMITTED"
                )];
                for t in &tasks {
                    lines.push(format!(
                        "{:<10} {:<15} {:<12} {}",
                        t.id, t.destination, t.status,
                        t.submitted_at.format("%H:%M:%S"),
                    ));
                }
                lines.join("\n")
            };
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }

        ClientMessage::TaskCancel { id } => {
            let data = match state.task_router.cancel_task(&id).await {
                Ok(task_router::CancelResult::Cancelled) => format!("Task '{}' cancelled.", id),
                Ok(task_router::CancelResult::AlreadyActive) => {
                    format!("Error: Task '{}' is already in progress — cannot cancel.", id)
                }
                Err(e) => format!("Error: {}", e),
            };
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }

        ClientMessage::TaskStatus { id } => {
            let data = match state.task_router.task_status(&id).await {
                Some(task) => format!(
                    "Task: {}\nAgent: {}\nStatus: {}\nSubmitted: {}\nContent: {}",
                    task.id, task.destination, task.status,
                    task.submitted_at.format("%Y-%m-%d %H:%M:%S"),
                    task.content,
                ),
                None => format!("Task '{}' not found.", id),
            };
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }

        ClientMessage::ContextSet { key, value } => {
            let data = match state.context.set(&key, &value, None) {
                Ok(()) => {
                    let _ = state.envelope.log_context_set(&key, &value);
                    format!("Set '{}'.", key)
                }
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

        ClientMessage::WorkflowRun { file, input } => {
            let parser = WorkflowParser::new();
            let dag = match parser.parse_file(&file) {
                Ok(d) => d,
                Err(e) => {
                    let data = format!("Error parsing workflow: {e}");
                    return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
                }
            };

            let run_id = uuid::Uuid::new_v4().to_string();
            let workflow_name = dag.definition.name.clone();
            let input_map: HashMap<String, serde_json::Value> = match input {
                Some(json) => serde_json::from_str(&json).unwrap_or_default(),
                None => HashMap::new(),
            };

            let engine = Arc::clone(&state.workflow_engine);
            let run_id_bg = run_id.clone();
            tokio::spawn(async move {
                if let Err(e) = engine.run(&dag, &run_id_bg, input_map).await {
                    tracing::error!(run_id = %run_id_bg, "Workflow run failed: {e}");
                }
            });

            let data = format!(
                "Workflow '{workflow_name}' started (run_id: {run_id})\nUse `bmux workflow status {run_id}` to check progress.",
            );
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }

        ClientMessage::WorkflowStatus { id } => {
            let data = match state.workflow_engine.get_status(&id) {
                Ok(Some(run)) => serde_json::to_string_pretty(&run)
                    .unwrap_or_else(|e| format!("Error serializing status: {e}")),
                Ok(None) => format!("Workflow run '{id}' not found."),
                Err(e) => format!("Error: {e}"),
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

/// Deliver sanitized task content to an agent's PTY pane.
async fn deliver_task_to_pane(
    state: &DaemonState,
    agent_id: &str,
    task_id: &str,
    content: &str,
) {
    let input = match state.envelope.sanitize_task_content(content) {
        Ok(bytes) => bytes,
        Err(e) => {
            state
                .task_router
                .fail_active_task_delivery(agent_id, &e.to_string())
                .await;
            return;
        }
    };

    let pane_id = {
        let reg = state.agents.lock().await;
        match reg.get(agent_id).and_then(|info| info.pane_id) {
            Some(id) => id,
            None => {
                state
                    .task_router
                    .fail_active_task_delivery(agent_id, "agent has no pane")
                    .await;
                return;
            }
        }
    };

    let delivered = {
        let mut sess = state.session.lock().await;
        sess.send_input_to_pane(pane_id, &input).is_ok()
    };

    if delivered {
        let _ = state.envelope.log_task_sent(agent_id, task_id, content);
    } else {
        state
            .task_router
            .fail_active_task_delivery(agent_id, "PTY write failed")
            .await;
    }
}

/// Subscribe to task dispatches and deliver content to agent PTY panes.
fn spawn_task_dispatch_subscriber(state: Arc<DaemonState>) {
    let mut rx = state.task_router.subscribe_dispatches();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    deliver_task_to_pane(
                        &state,
                        &event.agent_id,
                        &event.task_id,
                        &event.content,
                    )
                    .await;
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("Task dispatch subscriber lagged by {n} messages");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

/// Subscribe to IPC Result messages and advance the task lifecycle.
fn spawn_result_subscriber(router: Arc<TaskRouter>) {
    let mut rx = router.message_bus().subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) if msg.msg_type == MessageType::Result => {
                    match serde_json::from_value::<ResultPayload>(msg.payload) {
                        Ok(payload) => {
                            let task_full_id = payload
                                .task_id
                                .clone()
                                .unwrap_or_else(|| msg.id.clone());
                            if let Err(e) = router.handle_result(&task_full_id, payload).await {
                                tracing::warn!(
                                    task_id = %task_full_id,
                                    from = %msg.from,
                                    "Failed to handle agent result: {e}"
                                );
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                from = %msg.from,
                                "Malformed Result payload: {e}"
                            );
                        }
                    }
                }
                Ok(_) => {}
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("MessageBus subscriber lagged by {n} messages");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}
