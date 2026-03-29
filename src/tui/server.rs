/// Session daemon server (Story 1.5).
///
/// Invoked via `bmux --daemon --session <name>`.
/// - Manages session state (windows, panes, PTYs)
/// - Listens on `/tmp/bmux-<name>.sock` for client connections
/// - Writes metadata to `/tmp/bmux/<name>.json`
/// - Broadcasts screen-state updates to all connected clients
/// - Handles input and action messages from clients
use anyhow::Result;
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
    sync::{Mutex, broadcast},
    time::interval,
};

use super::{
    keybindings::Action,
    layout::ResizeDir,
    protocol::{
        ClientMessage, ServerMessage, SessionSnapshot,
    },
    session::{Session, SessionMeta, meta_path, socket_path},
};

pub struct DaemonServer {
    name: String,
    session: Arc<Mutex<Session>>,
    socket_path: PathBuf,
    meta_path: PathBuf,
}

impl DaemonServer {
    pub fn new(name: &str, rows: u16, cols: u16) -> Result<Self> {
        let session = Session::new(name, rows, cols)?;
        Ok(Self {
            name: name.to_string(),
            session: Arc::new(Mutex::new(session)),
            socket_path: socket_path(name),
            meta_path: meta_path(name),
        })
    }

    pub async fn run(self) -> Result<()> {
        // Remove stale socket if present
        let _ = std::fs::remove_file(&self.socket_path);

        let listener = UnixListener::bind(&self.socket_path)?;
        tracing::info!("Session '{}' listening on {:?}", self.name, self.socket_path);

        // Write session metadata
        self.write_meta().await?;

        // Broadcast channel: server sends SessionSnapshot to all clients
        let (tx, _) = broadcast::channel::<Arc<ServerMessage>>(32);
        let tx = Arc::new(tx);

        // Background task: periodically check for PTY output and broadcast state
        {
            let session_clone = Arc::clone(&self.session);
            let tx_clone = Arc::clone(&tx);
            tokio::spawn(async move {
                let mut tick = interval(Duration::from_millis(16)); // ~60fps max
                loop {
                    tick.tick().await;
                    let (has_output, snapshot) = {
                        let sess = session_clone.lock().await;
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
            let session_clone = Arc::clone(&self.session);
            let tx_clone = Arc::clone(&tx);

            tokio::spawn(async move {
                if let Err(e) = handle_client(stream, session_clone, tx_clone).await {
                    tracing::warn!("Client handler error: {e}");
                }
            });
        }
    }

    async fn write_meta(&self) -> Result<()> {
        let window_count = self.session.lock().await.window_count();
        let meta = SessionMeta::new(&self.name, window_count, self.socket_path.clone());
        let json = serde_json::to_string_pretty(&meta)?;
        tokio::fs::write(&self.meta_path, json).await?;
        Ok(())
    }
}

/// Handle one connected client for its lifetime.
async fn handle_client(
    stream: UnixStream,
    session: Arc<Mutex<Session>>,
    broadcast_tx: Arc<broadcast::Sender<Arc<ServerMessage>>>,
) -> Result<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut lines = BufReader::new(read_half).lines();
    let mut rx = broadcast_tx.subscribe();

    // Terminal size (negotiated via Init message)
    let mut term_rows: u16 = 24;
    let mut term_cols: u16 = 80;

    // Send initial state immediately after connect
    {
        let sess = session.lock().await;
        let snap = build_snapshot(&sess);
        let msg = serde_json::to_string(&ServerMessage::State(snap))? + "\n";
        write_half.write_all(msg.as_bytes()).await?;
    }

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
                                    &session,
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
                                        // CLI query — send response and close
                                        let json = serde_json::to_string(&response)? + "\n";
                                        write_half.write_all(json.as_bytes()).await?;
                                        // Don't break — client will close
                                    }
                                    HandleResult::Ok => {
                                        // TUI mutation — send fresh state snapshot
                                        let snap = build_snapshot(&*session.lock().await);
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
    session: &Arc<Mutex<Session>>,
    term_rows: &mut u16,
    term_cols: &mut u16,
) -> Result<HandleResult> {
    let mut sess = session.lock().await;
    match msg {
        ClientMessage::Init { rows, cols } | ClientMessage::Resize { rows, cols } => {
            *term_rows = rows;
            *term_cols = cols;
            sess.resize(rows, cols)?;
        }
        ClientMessage::Input { data } => {
            sess.send_input(&data)?;
        }
        ClientMessage::Action { action } => match action {
            Action::Detach => return Ok(HandleResult::Detach),
            Action::SplitHorizontal => {
                sess.active_window().split_horizontal()?;
            }
            Action::SplitVertical => {
                sess.active_window().split_vertical()?;
            }
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
                let _ = crate::config::settings::BmuxConfig::hot_reload(&path);
            }
        },
        ClientMessage::GetState => {} // state is sent after this function returns

        // ── CLI query handlers ────────────────────────────────────────────
        ClientMessage::AgentList => {
            // TODO: query the session's AgentRegistry and serialize
            let data = "No agents running. (Agent registry not yet integrated into daemon)".to_string();
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }
        ClientMessage::AgentStatus { name } => {
            let data = format!("Agent '{}': status query not yet integrated into daemon.", name);
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }
        ClientMessage::AgentSpawn { agent_type, name, model } => {
            let data = format!(
                "Spawning {} agent '{}' (model: {}) — not yet integrated into daemon.",
                agent_type, name, model.unwrap_or_default()
            );
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }
        ClientMessage::AgentKill { name } => {
            let data = format!("Kill agent '{}' — not yet integrated into daemon.", name);
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }
        ClientMessage::TaskList => {
            let data = "No tasks. (Task router not yet integrated into daemon)".to_string();
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }
        ClientMessage::TaskSend { agent, content, model: _ } => {
            let target = agent.unwrap_or_else(|| "auto".to_string());
            let data = format!("Task sent to '{}': {} — not yet dispatched (daemon integration pending).", target, content);
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }
        ClientMessage::TaskCancel { id } => {
            let data = format!("Cancel task '{}' — not yet integrated.", id);
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }
        ClientMessage::TaskStatus { id } => {
            let data = format!("Task '{}' — status query not yet integrated.", id);
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }
        ClientMessage::ContextSet { key, value } => {
            // TODO: write to session's context store
            let data = format!("Set '{}' = '{}' — context store not yet integrated into daemon.", key, value);
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }
        ClientMessage::ContextGet { key } => {
            let data = format!("Key '{}' — context store not yet integrated into daemon.", key);
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }
        ClientMessage::ContextList => {
            let data = "No context entries. (Context store not yet integrated into daemon)".to_string();
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }
        ClientMessage::ContextDump => {
            let data = "{}".to_string();
            return Ok(HandleResult::Reply(ServerMessage::QueryResult { data }));
        }
    }
    Ok(HandleResult::Ok)
}

// ── Snapshot builder ──────────────────────────────────────────────────────────

fn build_snapshot(sess: &Session) -> SessionSnapshot {
    sess.snapshot()
}
