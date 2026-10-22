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
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, Widget},
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
    ipc_client::{auth_for_socket, encode_signed_client, parse_signed_server},
    keybindings::{key_event_to_bytes, Action, KeybindingState},
    overlay::{render_help, Prompt, PromptKind, PromptOutcome},
    protocol::{ClientMessage, PaneSnapshot, ServerMessage, SessionSnapshot, WindowSnapshot},
    session::STATUS_FLASH_TTL,
    status_bar::{render_agent_badges, spans_width},
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
    let (server_err_tx, mut server_err_rx) = mpsc::unbounded_channel::<()>();

    // Server reader task
    let reader_auth = auth.clone();
    tokio::spawn(async move {
        let mut lines = BufReader::new(read_half).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) if !line.is_empty() => {
                    if let Ok(msg) = parse_signed_server(&reader_auth, &line) {
                        match msg {
                            ServerMessage::State(snap) => {
                                let _ = state_tx.send(snap);
                            }
                            ServerMessage::Detached => {
                                let _ = server_err_tx.send(());
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
                }
                Ok(None) | Err(_) => {
                    let _ = server_err_tx.send(());
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

    while running {
        // Drain latest state (use most recent snapshot)
        while let Ok(snap) = state_rx.try_recv() {
            current_snapshot = Some(snap);
        }
        while let Ok(reply) = reply_rx.try_recv() {
            ui.flash(summarize_reply(&reply));
        }

        // Check for server disconnect
        if server_err_rx.try_recv().is_ok() {
            break;
        }

        // Render
        if let Some(ref snap) = current_snapshot {
            let flash = ui.active_flash().map(str::to_string);
            let prefix_label = keybindings.prefix().label();
            terminal.draw(|f| {
                render_snapshot(f, snap, flash.as_deref());
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
    Ok(())
}

// ── Rendering from snapshot ───────────────────────────────────────────────────

fn render_snapshot(frame: &mut Frame, snap: &SessionSnapshot, flash: Option<&str>) {
    let total = frame.area();
    let status_rows = u16::from(snap.status_bar);

    let chunks = RLayout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(status_rows)])
        .split(total);

    let content_area = chunks[0];
    let status_area = chunks[1];

    // Find active window
    if let Some(win) = snap.windows.get(snap.active_window) {
        let buf = frame.buffer_mut();
        render_window(buf, content_area, win);
    }

    if status_rows == 0 {
        return;
    }
    // Status bar
    let bar = build_status_bar(snap, flash);
    let left_width = spans_width(&bar.spans);
    frame.render_widget(Paragraph::new(bar), status_area);
    render_agent_badges(&snap.agents, left_width, status_area, frame.buffer_mut());
}

fn render_window(buf: &mut Buffer, _area: Rect, win: &WindowSnapshot) {
    for layout_entry in &win.pane_layouts {
        let rect = Rect::new(
            layout_entry.x,
            layout_entry.y,
            layout_entry.width,
            layout_entry.height,
        );
        let focused = layout_entry.id == win.focused_pane;
        let border_style = if focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);
        let inner = block.inner(rect);
        block.render(rect, buf);

        // Find pane cells
        if let Some(pane_snap) = win.pane_cells.iter().find(|p| p.id == layout_entry.id) {
            render_pane_cells(buf, inner, pane_snap, focused);
        }
    }
}

fn render_pane_cells(buf: &mut Buffer, inner: Rect, snap: &PaneSnapshot, focused: bool) {
    for (row_idx, row) in snap.cells.iter().enumerate() {
        let y = inner.y + row_idx as u16;
        if y >= inner.y + inner.height {
            break;
        }
        for (col_idx, cell) in row.iter().enumerate() {
            let x = inner.x + col_idx as u16;
            if x >= inner.x + inner.width {
                break;
            }
            let is_cursor =
                focused && row_idx as u16 == snap.cursor_row && col_idx as u16 == snap.cursor_col;
            let style = if is_cursor {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                let fg = if cell.fg_r == 0 && cell.fg_g == 0 && cell.fg_b == 0 {
                    Color::Reset
                } else {
                    Color::Rgb(cell.fg_r, cell.fg_g, cell.fg_b)
                };
                let bg = if cell.bg_r == 0 && cell.bg_g == 0 && cell.bg_b == 0 {
                    Color::Reset
                } else {
                    Color::Rgb(cell.bg_r, cell.bg_g, cell.bg_b)
                };
                let mut s = Style::default().fg(fg).bg(bg);
                if cell.bold {
                    s = s.add_modifier(Modifier::BOLD);
                }
                s
            };
            let text = if cell.ch == '\0' { ' ' } else { cell.ch };
            buf.set_string(x, y, text.to_string(), style);
        }
    }
}

fn build_status_bar(snap: &SessionSnapshot, flash: Option<&str>) -> ratatui::text::Line<'static> {
    use crate::tui::scroll::SCROLL_MODE_INDICATOR;
    use ratatui::text::Line;
    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled(
        format!(" [{}] ", snap.name),
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD),
    ));
    for (i, w) in snap.windows.iter().enumerate() {
        let active = i == snap.active_window;
        let label = format!(" {} {} ", i + 1, w.name);
        if active {
            spans.push(Span::styled(
                label,
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                label,
                Style::default().fg(Color::White).bg(Color::DarkGray),
            ));
        }
        spans.push(Span::raw(" "));
    }

    if let Some(win) = snap.windows.get(snap.active_window) {
        if win.scroll_active {
            spans.push(Span::styled(
                format!(" {SCROLL_MODE_INDICATOR} "),
                Style::default().fg(Color::Black).bg(Color::Cyan),
            ));
        }
        if win.zoomed {
            spans.push(Span::styled(
                " [ZOOM] ",
                Style::default().fg(Color::Black).bg(Color::Magenta),
            ));
        }
    }

    if let Some(msg) = flash.or(snap.status_message.as_deref()) {
        let style = if msg.starts_with("Config error:") || msg.starts_with("Error") {
            Style::default().fg(Color::White).bg(Color::Red)
        } else {
            Style::default().fg(Color::Black).bg(Color::Blue)
        };
        spans.push(Span::styled(format!(" {msg} "), style));
    }

    Line::from(spans)
}
