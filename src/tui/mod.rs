pub mod client;
pub mod ipc_client;
pub mod keybindings;
pub mod layout;
pub mod overlay;
pub mod pane;
pub mod protocol;
pub mod pty_spawn;
pub mod scroll;
pub mod server;
pub mod session;
pub mod session_auth;
pub mod status_bar;
pub mod window;
pub mod zoom;

use anyhow::Result;
use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, layout::Rect, Terminal};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::config::settings::BmuxConfig;
use keybindings::{key_event_to_bytes, Action, KeybindingState};
use layout::ResizeDir;
use overlay::{render_help, Prompt, PromptKind, PromptOutcome};
use session::Session;
use status_bar::StatusMessage;

/// Tracks mouse drag state for pane border resizing.
struct DragState {
    /// If dragging, the left pane ID of the border being dragged.
    left_pane_id: Option<usize>,
}

/// State of the local TUI loop.
struct LocalUi {
    keybindings: KeybindingState,
    drag: DragState,
    prompt: Option<Prompt>,
    help: bool,
    running: bool,
    rows: u16,
    cols: u16,
}

/// Apply the `[tui]` settings to a local session.
fn apply_local_config(session: &mut Session, cfg: &BmuxConfig) -> Result<()> {
    pane::set_default_shell(Some(cfg.shell().to_string()));
    session.set_status_bar_visible(cfg.tui.status_bar)
}

/// Run the TUI in local (single-process) mode.
///
/// Used when `bmux` is invoked with no arguments.
/// All windows and panes live in this process — no daemon needed.
pub async fn run_local(session_name: &str) -> Result<()> {
    let (cols, rows) = crossterm::terminal::size()?;

    // Enter raw mode and alternate screen buffer
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Apply the configured shell before the first pane is created.
    let config = BmuxConfig::load().unwrap_or_default();
    pane::set_default_shell(Some(config.shell().to_string()));
    let mut session = Session::new(session_name, rows, cols)?;
    apply_local_config(&mut session, &config)?;
    let mut ui = LocalUi {
        keybindings: KeybindingState::from_config(config.prefix_key()),
        drag: DragState { left_pane_id: None },
        prompt: None,
        help: false,
        running: true,
        rows,
        cols,
    };

    // Async keyboard event channel (spawn_blocking for crossterm reads)
    let (key_tx, mut key_rx) = mpsc::unbounded_channel::<Event>();
    // The loop ends when the receiver is dropped, so the runtime can shut
    // down after detach instead of waiting for this thread forever.
    tokio::task::spawn_blocking(move || loop {
        if key_tx.is_closed() {
            break;
        }
        if crossterm::event::poll(Duration::from_millis(50)).unwrap_or(false) {
            if let Ok(ev) = crossterm::event::read() {
                if key_tx.send(ev).is_err() {
                    break;
                }
            }
        }
    });

    // Render interval: up to 60 fps
    let mut render_ticker = tokio::time::interval(Duration::from_millis(16));

    while ui.running {
        tokio::select! {
            _ = render_ticker.tick() => {
                session.expire_status_flash();
                let prefix_label = ui.keybindings.prefix().label();
                terminal.draw(|f| {
                    session.render(f);
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

            maybe_event = async {
                // Drain the key channel (non-blocking try_recv)
                key_rx.recv().await
            } => {
                if let Some(ev) = maybe_event {
                    handle_event(ev, &mut session, &mut ui)?;
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(())
}

fn handle_event(event: Event, session: &mut Session, ui: &mut LocalUi) -> Result<()> {
    match event {
        Event::Key(_) if ui.help => ui.help = false,
        Event::Key(key) if ui.prompt.is_some() => {
            let prompt = ui.prompt.as_mut().expect("prompt is open");
            match prompt.handle_key(key) {
                PromptOutcome::Pending => {}
                PromptOutcome::Cancelled => ui.prompt = None,
                PromptOutcome::Submitted(text) => {
                    if ui.prompt.take().map(|p| p.kind) == Some(PromptKind::RenameWindow) {
                        session.rename_active_window(&text);
                    }
                }
            }
        }
        Event::Key(key) => {
            let (action, pass_to_pty) = ui.keybindings.process(key);
            if let Some(act) = action {
                handle_action(act, session, ui)?;
            } else if pass_to_pty {
                let bytes = key_event_to_bytes(key);
                if !bytes.is_empty() && !session.handle_scroll_input(&bytes) {
                    session.send_input(&bytes)?;
                }
            }
        }
        Event::Mouse(mouse) => {
            handle_mouse(mouse, session, &mut ui.drag);
        }
        Event::Resize(new_cols, new_rows) => {
            ui.rows = new_rows;
            ui.cols = new_cols;
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
        MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Moved => {
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

fn handle_action(action: Action, session: &mut Session, ui: &mut LocalUi) -> Result<()> {
    match action {
        Action::Detach => ui.running = false,
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
        Action::NewWindow => session.create_window(ui.rows, ui.cols)?,
        Action::NextWindow => session.next_window(),
        Action::PrevWindow => session.prev_window(),
        Action::SwitchWindow(n) => session.switch_to_window(n),
        Action::FocusNextPane => session.active_window().focus_next_pane(),
        Action::ToggleZoom => session.toggle_zoom(),
        Action::EnterScrollMode => session.enter_scroll_mode(),
        Action::ReloadConfig => {
            let path = crate::config::default_config_path();
            match BmuxConfig::hot_reload(&path) {
                Ok(cfg) => {
                    apply_local_config(session, &cfg)?;
                    ui.keybindings = KeybindingState::from_config(cfg.prefix_key());
                    session.set_status_flash(StatusMessage::ConfigReloaded);
                }
                Err(e) => session.set_status_flash(StatusMessage::ConfigError(e)),
            }
        }
        Action::RenameWindow => {
            let current = session.active_window_name().to_string();
            ui.prompt = Some(Prompt::with_text(PromptKind::RenameWindow, &current));
        }
        Action::SendTask | Action::SendTaskAuto => {
            session.set_status_flash(StatusMessage::Custom(
                "Agents run in sessions. Start one with `bmux new <name>`.".to_string(),
            ));
        }
        Action::ShowHelp => ui.help = true,
    }
    Ok(())
}
