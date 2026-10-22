/// TUI client — connects to a session daemon and renders its state locally.
///
/// Used for `bmux new <name>` and `bmux attach -s <name>`.
use anyhow::{Context, Result};
use crossterm::{
    event::Event,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout as RLayout, Rect},
    widgets::Widget,
    Frame, Terminal,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    sync::mpsc,
};

use super::{
    chrome::{cursor_position, pane_block, pane_title, render_pane_cells},
    ipc_client::{auth_for_socket, encode_signed_client, parse_signed_server},
    keybindings::{key_event_to_bytes, Action, KeybindingState},
    overlay::{render_help, Prompt, PromptKind, PromptOutcome},
    protocol::{AgentBadge, ClientMessage, ServerMessage, SessionSnapshot, WindowSnapshot},
    session::STATUS_FLASH_TTL,
    status_bar::{StatusBar, StatusBarState, StatusMessage},
    zoom::ZoomState,
};
use crate::config::settings::BmuxConfig;

/// Client-side UI state that the daemon does not know about.
#[derive(Default)]
struct ClientUi {
    prompt: Option<Prompt>,
    help: bool,
    /// Message shown in the status bar (e.g. the reply to a task send).
    flash: Option<(String, Instant)>,
}

impl ClientUi {
    fn flash(&mut self, msg: impl Into<String>) {
        self.flash = Some((msg.into(), Instant::now()));
    }

    fn active_flash(&mut self) -> Option<&str> {
        if matches!(&self.flash, Some((_, at)) if at.elapsed() >= STATUS_FLASH_TTL) {
            self.flash = None;
        }
        self.flash.as_ref().map(|(m, _)| m.as_str())
    }
}

/// Agent running in the focused pane of the active window, if any.
fn focused_agent(snap: &SessionSnapshot) -> Option<String> {
    let win = snap.windows.get(snap.active_window)?;
    snap.agents
        .iter()
        .find(|a| a.pane_id == Some(win.focused_pane))
        .map(|a| a.name.clone())
}

/// First line of a server reply, short enough for the status bar.
fn summarize_reply(data: &str) -> String {
    let line = data.lines().next().unwrap_or_default();
    let mut out: String = line.chars().take(120).collect();
    if line.chars().count() > 120 {
        out.push('…');
    }
    out
}

/// Bad frames in a row, after a good one, before the client gives up.
const MAX_BAD_FRAMES: u32 = 3;

const UNREADABLE_DAEMON: &str = "Cannot read messages from the session daemon. \
     It may be running a different bmux version. Stop it with `bmux kill <session>` \
     and start the session again.";

pub async fn run_client(socket: PathBuf) -> Result<()> {
    let auth = auth_for_socket(&socket)?;
    let stream = UnixStream::connect(&socket)
        .await
        .with_context(|| format!("Cannot connect to session socket {:?}", socket))?;

    let (read_half, mut write_half) = stream.into_split();

    // Get terminal size before entering raw mode
    let (cols, rows) = crossterm::terminal::size()?;

    // Setup TUI
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Send Init (signed)
    write_half
        .write_all(encode_signed_client(&auth, ClientMessage::Init { rows, cols })?.as_bytes())
        .await?;

    // Channel: server-reader thread → main loop
    let (state_tx, mut state_rx) = mpsc::unbounded_channel::<SessionSnapshot>();
    let (reply_tx, mut reply_rx) = mpsc::unbounded_channel::<String>();
    // `Some(reason)` when the client gives up on the daemon's messages.
    let (server_err_tx, mut server_err_rx) = mpsc::unbounded_channel::<Option<String>>();

    // Server reader task
    let reader_auth = auth.clone();
    tokio::spawn(async move {
        let mut lines = BufReader::new(read_half).lines();
        let mut bad_frames = 0u32;
        let mut read_any = false;
        loop {
            match lines.next_line().await {
                Ok(Some(line)) if !line.is_empty() => {
                    let msg = match parse_signed_server(&reader_auth, &line) {
                        Ok(msg) => {
                            bad_frames = 0;
                            read_any = true;
                            msg
                        }
                        Err(e) => {
                            // Not `warn`: stderr is the TUI screen.
                            tracing::debug!("Dropped a server frame: {e}");
                            bad_frames += 1;
                            // The daemon sends state only when it changes, so
                            // an idle session may send just one frame.
                            if !read_any || bad_frames >= MAX_BAD_FRAMES {
                                let _ = server_err_tx.send(Some(UNREADABLE_DAEMON.to_string()));
                                break;
                            }
                            continue;
                        }
                    };
                    match msg {
                        ServerMessage::State(snap) => {
                            let _ = state_tx.send(snap);
                        }
                        ServerMessage::Detached => {
                            let _ = server_err_tx.send(None);
                            break;
                        }
                        ServerMessage::Error { msg } => {
                            tracing::warn!("Server error: {msg}");
                        }
                        ServerMessage::QueryResult { data } => {
                            // Reply to a request made from the TUI (task send).
                            let _ = reply_tx.send(data);
                        }
                    }
                }
                Ok(None) | Err(_) => {
                    let _ = server_err_tx.send(None);
                    break;
                }
                _ => {}
            }
        }
    });

    // Event reader task (blocking crossterm reads → async channel)
    let (key_tx, mut key_rx) = mpsc::unbounded_channel::<Event>();
    // The loop ends when the receiver is dropped, so the runtime can shut
    // down after detach instead of waiting for this thread forever.
    tokio::task::spawn_blocking(move || loop {
        if key_tx.is_closed() {
            break;
        }
        if crossterm::event::poll(Duration::from_millis(100)).unwrap_or(false) {
            if let Ok(ev) = crossterm::event::read() {
                if key_tx.send(ev).is_err() {
                    break;
                }
            }
        }
    });

    let mut current_snapshot: Option<SessionSnapshot> = None;
    let config = BmuxConfig::load().unwrap_or_default();
    let mut keybindings = KeybindingState::from_config(config.prefix_key());
    let mut ui = ClientUi::default();
    let mut running = true;
    let mut exit_error: Option<String> = None;

    while running {
        // Drain latest state (use most recent snapshot)
        while let Ok(snap) = state_rx.try_recv() {
            current_snapshot = Some(snap);
        }
        while let Ok(reply) = reply_rx.try_recv() {
            ui.flash(summarize_reply(&reply));
        }

        // Check for server disconnect
        if let Ok(reason) = server_err_rx.try_recv() {
            exit_error = reason;
            break;
        }

        // Render
        if let Some(ref snap) = current_snapshot {
            let flash = ui.active_flash().map(str::to_string);
            let prefix_label = keybindings.prefix().label();
            let prefix_active = keybindings.is_prefix_active();
            let overlay_open = ui.prompt.is_some() || ui.help;
            terminal.draw(|f| {
                let cursor =
                    render_snapshot(f, snap, flash.as_deref(), prefix_active, &prefix_label);
                if let (Some((x, y)), false) = (cursor, overlay_open) {
                    f.set_cursor_position((x, y));
                }
                let area = f.area();
                if let Some(prompt) = &ui.prompt {
                    let row = Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1);
                    prompt.render(row, f.buffer_mut());
                }
                if ui.help {
                    render_help(area, f.buffer_mut(), &prefix_label);
                }
            })?;
        }

        // Handle keyboard event (non-blocking)
        if let Ok(ev) = key_rx.try_recv() {
            match ev {
                Event::Key(_) if ui.help => ui.help = false,
                Event::Key(key) if ui.prompt.is_some() => {
                    let prompt = ui.prompt.as_mut().expect("prompt is open");
                    match prompt.handle_key(key) {
                        PromptOutcome::Pending => {}
                        PromptOutcome::Cancelled => ui.prompt = None,
                        PromptOutcome::Submitted(text) => {
                            let kind = ui.prompt.take().expect("prompt is open").kind;
                            let msg = match kind {
                                PromptKind::RenameWindow => {
                                    ClientMessage::RenameWindow { name: text }
                                }
                                PromptKind::SendTask { agent } => ClientMessage::TaskSend {
                                    agent: Some(agent),
                                    content: text,
                                    model: None,
                                },
                                PromptKind::SendTaskAuto => ClientMessage::TaskSend {
                                    agent: None,
                                    content: text,
                                    model: None,
                                },
                            };
                            write_half
                                .write_all(encode_signed_client(&auth, msg)?.as_bytes())
                                .await?;
                        }
                    }
                }
                Event::Key(key) => {
                    let (action, pass_to_pty) = keybindings.process(key);
                    if let Some(act) = action {
                        match act {
                            Action::Detach => running = false,
                            Action::ShowHelp => {
                                ui.help = true;
                                continue;
                            }
                            Action::RenameWindow => {
                                let current = current_snapshot
                                    .as_ref()
                                    .and_then(|s| s.windows.get(s.active_window))
                                    .map(|w| w.name.clone())
                                    .unwrap_or_default();
                                ui.prompt =
                                    Some(Prompt::with_text(PromptKind::RenameWindow, &current));
                                continue;
                            }
                            Action::SendTask => {
                                match current_snapshot.as_ref().and_then(focused_agent) {
                                    Some(agent) => {
                                        ui.prompt =
                                            Some(Prompt::new(PromptKind::SendTask { agent }))
                                    }
                                    None => ui.flash(format!(
                                        "Focused pane is not an agent. Use {} T to auto-route.",
                                        keybindings.prefix().label()
                                    )),
                                }
                                continue;
                            }
                            Action::SendTaskAuto => {
                                let has_agents = current_snapshot
                                    .as_ref()
                                    .is_some_and(|s| !s.agents.is_empty());
                                if has_agents {
                                    ui.prompt = Some(Prompt::new(PromptKind::SendTaskAuto));
                                } else {
                                    ui.flash("No agents in this session. Start one with `bmux agent spawn`.");
                                }
                                continue;
                            }
                            Action::ReloadConfig => {
                                // The prefix key is read by the client, the rest by the daemon.
                                if let Ok(cfg) =
                                    BmuxConfig::hot_reload(&crate::config::default_config_path())
                                {
                                    keybindings = KeybindingState::from_config(cfg.prefix_key());
                                }
                            }
                            _ => {}
                        }
                        write_half
                            .write_all(
                                encode_signed_client(&auth, ClientMessage::Action { action: act })?
                                    .as_bytes(),
                            )
                            .await?;
                    } else if pass_to_pty {
                        let bytes = key_event_to_bytes(key);
                        if !bytes.is_empty() {
                            write_half
                                .write_all(
                                    encode_signed_client(
                                        &auth,
                                        ClientMessage::Input { data: bytes },
                                    )?
                                    .as_bytes(),
                                )
                                .await?;
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    use crossterm::event::{MouseButton, MouseEventKind};
                    match mouse.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            write_half
                                .write_all(
                                    encode_signed_client(
                                        &auth,
                                        ClientMessage::MouseClick {
                                            col: mouse.column,
                                            row: mouse.row,
                                        },
                                    )?
                                    .as_bytes(),
                                )
                                .await?;
                        }
                        MouseEventKind::Drag(MouseButton::Left) => {
                            write_half
                                .write_all(
                                    encode_signed_client(
                                        &auth,
                                        ClientMessage::MouseDrag {
                                            col: mouse.column,
                                            row: mouse.row,
                                        },
                                    )?
                                    .as_bytes(),
                                )
                                .await?;
                        }
                        MouseEventKind::Moved => {
                            write_half
                                .write_all(
                                    encode_signed_client(
                                        &auth,
                                        ClientMessage::MouseDrag {
                                            col: mouse.column,
                                            row: mouse.row,
                                        },
                                    )?
                                    .as_bytes(),
                                )
                                .await?;
                        }
                        MouseEventKind::Up(MouseButton::Left) => {
                            write_half
                                .write_all(
                                    encode_signed_client(&auth, ClientMessage::MouseUp)?.as_bytes(),
                                )
                                .await?;
                        }
                        _ => {}
                    }
                }
                Event::Resize(cols, rows) => {
                    write_half
                        .write_all(
                            encode_signed_client(&auth, ClientMessage::Resize { rows, cols })?
                                .as_bytes(),
                        )
                        .await?;
                }
                _ => {}
            }
        }

        tokio::time::sleep(Duration::from_millis(8)).await;
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    match exit_error {
        Some(reason) => Err(anyhow::anyhow!(reason)),
        None => Ok(()),
    }
}

// ── Rendering from snapshot ───────────────────────────────────────────────────

/// Draw a session snapshot. Returns the cursor position of the focused
/// pane when its application shows a cursor and it is not in scroll mode.
fn render_snapshot(
    frame: &mut Frame,
    snap: &SessionSnapshot,
    flash: Option<&str>,
    prefix_active: bool,
    prefix_label: &str,
) -> Option<(u16, u16)> {
    let total = frame.area();
    let status_rows = u16::from(snap.status_bar);

    let chunks = RLayout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(status_rows)])
        .split(total);

    let content_area = chunks[0];
    let status_area = chunks[1];

    let mut cursor = None;
    if let Some(win) = snap.windows.get(snap.active_window) {
        cursor = render_window(frame.buffer_mut(), content_area, win, &snap.agents);
    }

    if status_rows > 0 {
        let state = status_bar_state(snap, flash, prefix_active, prefix_label);
        frame.render_widget(StatusBar::new(&state), status_area);
    }
    cursor
}

/// Status bar state for a daemon snapshot (same widget as local mode).
fn status_bar_state(
    snap: &SessionSnapshot,
    flash: Option<&str>,
    prefix_active: bool,
    prefix_label: &str,
) -> StatusBarState {
    let win = snap.windows.get(snap.active_window);
    let mut zoom = ZoomState::new();
    if let Some(w) = win.filter(|w| w.zoomed) {
        zoom.toggle(w.focused_pane);
    }
    let message = flash
        .or(snap.status_message.as_deref())
        .map(|m| StatusMessage::Custom(m.to_string()))
        .unwrap_or(StatusMessage::None);
    StatusBarState {
        session_name: snap.name.clone(),
        window_labels: snap.windows.iter().map(|w| w.name.clone()).collect(),
        active_window_idx: snap.active_window,
        in_scroll_mode: win.is_some_and(|w| w.scroll_active),
        zoom,
        message,
        agents: snap.agents.clone(),
        prefix_active,
        prefix_label: prefix_label.to_string(),
    }
}

fn render_window(
    buf: &mut Buffer,
    area: Rect,
    win: &WindowSnapshot,
    agents: &[AgentBadge],
) -> Option<(u16, u16)> {
    let mut cursor = None;
    for layout_entry in &win.pane_layouts {
        let rect = Rect::new(
            layout_entry.x,
            layout_entry.y,
            layout_entry.width,
            layout_entry.height,
        )
        .intersection(area.intersection(buf.area));
        // A zoomed window sends only the zoomed pane, which has the focus.
        let focused = win.zoomed || layout_entry.id == win.focused_pane;
        let mut block = pane_block(focused);
        if let Some(title) = agents
            .iter()
            .find(|a| a.pane_id == Some(layout_entry.id))
            .and_then(|a| pane_title(a, focused, rect.width))
        {
            block = block.title(title);
        }
        let inner = block.inner(rect);
        block.render(rect, buf);

        if let Some(pane_snap) = win.pane_cells.iter().find(|p| p.id == layout_entry.id) {
            render_pane_cells(buf, inner, pane_snap);
            if focused && !win.scroll_active {
                cursor = cursor_position(inner, pane_snap);
            }
        }
    }
    cursor
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::protocol::{CellData, PaneLayout, PaneSnapshot};
    use ratatui::{backend::TestBackend, Terminal};

    fn snapshot(cursor_visible: bool) -> SessionSnapshot {
        let mut cells = vec![vec![CellData::default(); 8]; 2];
        cells[0][0].ch = "h".to_string();
        SessionSnapshot {
            name: "demo".to_string(),
            active_window: 0,
            windows: vec![WindowSnapshot {
                id: 0,
                name: "parser".to_string(),
                pane_layouts: vec![PaneLayout {
                    id: 3,
                    x: 0,
                    y: 0,
                    width: 40,
                    height: 4,
                }],
                pane_cells: vec![PaneSnapshot {
                    id: 3,
                    rows: 2,
                    cols: 8,
                    cells,
                    cursor_row: 1,
                    cursor_col: 2,
                    cursor_visible,
                }],
                focused_pane: 3,
                zoomed: false,
                scroll_active: false,
            }],
            status_message: None,
            agents: vec![AgentBadge {
                name: "dev".to_string(),
                agent_type: "claude-code".to_string(),
                model: "claude-sonnet-4-5".to_string(),
                status: "working".to_string(),
                tokens: 0,
                cost_usd: 0.14,
                cost_tracked: true,
                pane_id: Some(3),
            }],
            status_bar: true,
        }
    }

    fn draw(snap: &SessionSnapshot) -> (Buffer, Option<(u16, u16)>) {
        let mut terminal = Terminal::new(TestBackend::new(60, 5)).unwrap();
        let mut cursor = None;
        terminal
            .draw(|f| cursor = render_snapshot(f, snap, None, false, "Ctrl-b"))
            .unwrap();
        (terminal.backend().buffer().clone(), cursor)
    }

    fn row(buf: &Buffer, y: u16) -> String {
        (0..buf.area.width)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect()
    }

    #[test]
    fn agent_pane_gets_rounded_border_and_title() {
        let (buf, _) = draw(&snapshot(true));
        let top = row(&buf, 0);
        assert!(top.starts_with("╭ ● dev · sonnet-4-5 · $0.14 "), "{top:?}");
        assert_eq!(buf[(1, 1)].symbol(), "h");
        let bar = row(&buf, 4);
        assert!(bar.starts_with(" demo   1 parser "), "{bar:?}");
    }

    #[test]
    fn cursor_follows_application_visibility() {
        let (_, cursor) = draw(&snapshot(true));
        assert_eq!(cursor, Some((3, 2)));
        let (_, cursor) = draw(&snapshot(false));
        assert_eq!(cursor, None);
    }
}
