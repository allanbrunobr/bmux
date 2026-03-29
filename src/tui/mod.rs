pub mod client;
pub mod ipc_client;
pub mod keybindings;
pub mod layout;
pub mod pane;
pub mod protocol;
pub mod scroll;
pub mod server;
pub mod session;
pub mod status_bar;
pub mod window;
pub mod zoom;

use anyhow::Result;
use crossterm::{
    event::{Event, MouseEvent, MouseEventKind, MouseButton, EnableMouseCapture, DisableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::time::Duration;
use tokio::sync::mpsc;

use keybindings::{Action, KeybindingState, key_event_to_bytes};
use layout::ResizeDir;
use session::Session;

/// Tracks mouse drag state for pane border resizing.
struct DragState {
    /// If dragging, the left pane ID of the border being dragged.
    left_pane_id: Option<usize>,
}

/// Run the TUI in local (single-process) mode.
///
/// Used when `bmux` is invoked with no arguments (Story 1.1).
/// All windows and panes live in this process — no daemon needed.
pub async fn run_local(session_name: &str) -> Result<()> {
    let (cols, rows) = crossterm::terminal::size()?;

    // Enter raw mode and alternate screen buffer
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut session = Session::new(session_name, rows, cols)?;
    let mut keybindings = KeybindingState::new();
    let mut drag = DragState { left_pane_id: None };
    let mut running = true;

    // Async keyboard event channel (spawn_blocking for crossterm reads)
    let (key_tx, mut key_rx) = mpsc::unbounded_channel::<Event>();
    tokio::task::spawn_blocking(move || loop {
        if crossterm::event::poll(Duration::from_millis(50)).unwrap_or(false) {
            if let Ok(ev) = crossterm::event::read() {
                if key_tx.send(ev).is_err() {
                    break;
                }
            }
        }
    });

    // Render interval: up to 60 fps
    let mut render_ticker =
        tokio::time::interval(Duration::from_millis(16));

    while running {
        tokio::select! {
            _ = render_ticker.tick() => {
                // Only re-draw when PTY has new output or always at 60fps
                terminal.draw(|f| session.render(f))?;
            }

            maybe_event = async {
                // Drain the key channel (non-blocking try_recv)
                key_rx.recv().await
            } => {
                if let Some(ev) = maybe_event {
                    handle_event(
                        ev,
                        &mut session,
                        &mut keybindings,
                        &mut drag,
                        &mut running,
                        rows,
                        cols,
                    )?;
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

fn handle_event(
    event: Event,
    session: &mut Session,
    keybindings: &mut KeybindingState,
    drag: &mut DragState,
    running: &mut bool,
    rows: u16,
    cols: u16,
) -> Result<()> {
    match event {
        Event::Key(key) => {
            let (action, pass_to_pty) = keybindings.process(key);
            if let Some(act) = action {
                handle_action(act, session, running, rows, cols)?;
            } else if pass_to_pty {
                let bytes = key_event_to_bytes(key);
                if !bytes.is_empty() {
                    session.send_input(&bytes)?;
                }
            }
        }
        Event::Mouse(mouse) => {
            handle_mouse(mouse, session, drag);
        }
        Event::Resize(new_cols, new_rows) => {
            session.resize(new_rows, new_cols)?;
        }
        _ => {}
    }
    Ok(())
}

fn handle_mouse(mouse: MouseEvent, session: &mut Session, drag: &mut DragState) {
    // Always update hover position for border highlighting
    session.active_window().set_hover(mouse.column, mouse.row);

    match mouse.kind {
        // Click to focus pane
        MouseEventKind::Down(MouseButton::Left) => {
            let col = mouse.column;
            let row = mouse.row;

            // Check if clicking on a border (start drag)
            if let Some((left_id, _right_id, _border_x)) = session.border_at(col, row) {
                drag.left_pane_id = Some(left_id);
            } else {
                // Click on a pane — set focus
                session.focus_pane_at(col, row);
            }
        }

        // Drag to resize (Drag on iTerm2/Warp, Moved on Terminal.app)
        MouseEventKind::Drag(MouseButton::Left)
        | MouseEventKind::Moved => {
            if let Some(left_id) = drag.left_pane_id {
                session.active_window().drag_border(left_id, mouse.column);
            }
        }

        // Release drag
        MouseEventKind::Up(MouseButton::Left) => {
            drag.left_pane_id = None;
        }

        _ => {}
    }
}

fn handle_action(
    action: Action,
    session: &mut Session,
    running: &mut bool,
    rows: u16,
    cols: u16,
) -> Result<()> {
    match action {
        Action::Detach => *running = false,
        Action::SplitHorizontal => {
            session.active_window().split_horizontal()?;
        }
        Action::SplitVertical => {
            session.active_window().split_vertical()?;
        }
        Action::FocusLeft => session.active_window().focus_left(),
        Action::FocusRight => session.active_window().focus_right(),
        Action::FocusUp => session.active_window().focus_up(),
        Action::FocusDown => session.active_window().focus_down(),
        Action::ResizeLeft => session.active_window().resize_pane(ResizeDir::Left),
        Action::ResizeRight => session.active_window().resize_pane(ResizeDir::Right),
        Action::ResizeUp => session.active_window().resize_pane(ResizeDir::Up),
        Action::ResizeDown => session.active_window().resize_pane(ResizeDir::Down),
        Action::NewWindow => session.create_window(rows, cols)?,
        Action::NextWindow => session.next_window(),
        Action::PrevWindow => session.prev_window(),
        Action::SwitchWindow(n) => session.switch_to_window(n),
        Action::FocusNextPane => session.active_window().focus_next_pane(),
        Action::ToggleZoom => { /* TODO: integrate ZoomState with Window */ }
        Action::EnterScrollMode => { /* TODO: integrate ScrollState with Pane */ }
        Action::ReloadConfig => {
            // Story 1.8: hot reload config.toml
            let path = crate::config::default_config_path();
            match crate::config::settings::BmuxConfig::hot_reload(&path) {
                Ok(_cfg) => { /* TODO: apply new config, show "Config reloaded" in status bar */ }
                Err(_e) => { /* TODO: show "Config error: {e}" in status bar */ }
            }
        }
    }
    Ok(())
}
