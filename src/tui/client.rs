/// TUI client — connects to a session daemon and renders its state locally.
///
/// Used for `bmux new <name>` and `bmux attach -s <name>`.
use anyhow::{Context, Result};
use crossterm::{
    event::Event,
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout as RLayout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, Widget},
};
use std::{path::PathBuf, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    sync::mpsc,
};

use super::{
    keybindings::{Action, KeybindingState, key_event_to_bytes},
    protocol::{
        ClientMessage, PaneSnapshot, ServerMessage, SessionSnapshot,
        WindowSnapshot,
    },
};

pub async fn run_client(socket: PathBuf) -> Result<()> {
    let stream = UnixStream::connect(&socket)
        .await
        .with_context(|| format!("Cannot connect to session socket {:?}", socket))?;

    let (read_half, mut write_half) = stream.into_split();

    // Get terminal size before entering raw mode
    let (cols, rows) = crossterm::terminal::size()?;

    // Setup TUI
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Send Init
    let init_msg =
        serde_json::to_string(&ClientMessage::Init { rows, cols })? + "\n";
    write_half.write_all(init_msg.as_bytes()).await?;

    // Channel: server-reader thread → main loop
    let (state_tx, mut state_rx) = mpsc::unbounded_channel::<SessionSnapshot>();
    let (server_err_tx, mut server_err_rx) = mpsc::unbounded_channel::<()>();

    // Server reader task
    tokio::spawn(async move {
        let mut lines = BufReader::new(read_half).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) if !line.is_empty() => {
                    if let Ok(msg) = serde_json::from_str::<ServerMessage>(&line) {
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
    tokio::task::spawn_blocking(move || loop {
        if crossterm::event::poll(Duration::from_millis(100)).unwrap_or(false) {
            if let Ok(ev) = crossterm::event::read() {
                if key_tx.send(ev).is_err() {
                    break;
                }
            }
        }
    });

    let mut current_snapshot: Option<SessionSnapshot> = None;
    let mut keybindings = KeybindingState::new();
    let mut running = true;

    while running {
        // Drain latest state (use most recent snapshot)
        while let Ok(snap) = state_rx.try_recv() {
            current_snapshot = Some(snap);
        }

        // Check for server disconnect
        if server_err_rx.try_recv().is_ok() {
            break;
        }

        // Render
        if let Some(ref snap) = current_snapshot {
            let snap_clone = snap.clone();
            terminal.draw(|f| render_snapshot(f, &snap_clone))?;
        }

        // Handle keyboard event (non-blocking)
        if let Ok(ev) = key_rx.try_recv() {
            match ev {
                Event::Key(key) => {
                    let (action, pass_to_pty) = keybindings.process(key);
                    if let Some(act) = action {
                        if matches!(act, Action::Detach) {
                            running = false;
                        }
                        let msg =
                            serde_json::to_string(&ClientMessage::Action { action: act })?
                                + "\n";
                        write_half.write_all(msg.as_bytes()).await?;
                    } else if pass_to_pty {
                        let bytes = key_event_to_bytes(key);
                        if !bytes.is_empty() {
                            let msg =
                                serde_json::to_string(&ClientMessage::Input { data: bytes })?
                                    + "\n";
                            write_half.write_all(msg.as_bytes()).await?;
                        }
                    }
                }
                Event::Resize(cols, rows) => {
                    let msg =
                        serde_json::to_string(&ClientMessage::Resize { rows, cols })? + "\n";
                    write_half.write_all(msg.as_bytes()).await?;
                }
                _ => {}
            }
        }

        tokio::time::sleep(Duration::from_millis(8)).await;
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

// ── Rendering from snapshot ───────────────────────────────────────────────────

fn render_snapshot(frame: &mut Frame, snap: &SessionSnapshot) {
    let total = frame.area();

    let chunks = RLayout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(total);

    let content_area = chunks[0];
    let status_area = chunks[1];

    // Find active window
    if let Some(win) = snap.windows.get(snap.active_window) {
        let buf = frame.buffer_mut();
        render_window(buf, content_area, win);
    }

    // Status bar
    let bar = build_status_bar(snap);
    frame.render_widget(Paragraph::new(bar), status_area);
}

fn render_window(buf: &mut Buffer, _area: Rect, win: &WindowSnapshot) {
    for layout_entry in &win.pane_layouts {
        let rect = Rect::new(layout_entry.x, layout_entry.y, layout_entry.width, layout_entry.height);
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
            let is_cursor = focused
                && row_idx as u16 == snap.cursor_row
                && col_idx as u16 == snap.cursor_col;
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

fn build_status_bar(snap: &SessionSnapshot) -> ratatui::text::Line<'static> {
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
    Line::from(spans)
}
