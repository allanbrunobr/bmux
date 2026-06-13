use anyhow::Result;
use chrono::{DateTime, Utc};
use crate::security::agent_spawn::AgentPtySpawn;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout as RatatuiLayout},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{
    protocol::SessionSnapshot,
    status_bar::{StatusBar, StatusBarState, StatusMessage},
    window::Window,
};

/// In-process session — owns all windows and panes directly.
///
/// Used for `bmux` (no args) mode where no daemon is required.
pub struct Session {
    pub name: String,
    windows: Vec<Window>,
    active_window: usize,
    created_at: DateTime<Utc>,
    next_window_id: usize,
    status_flash: Option<StatusMessage>,
}

impl Session {
    pub fn new(name: &str, rows: u16, cols: u16) -> Result<Self> {
        // Reserve 1 row for the status bar
        let content_rows = rows.saturating_sub(1).max(1);
        let window = Window::new(0, content_rows, cols)?;
        Ok(Self {
            name: name.to_string(),
            windows: vec![window],
            active_window: 0,
            created_at: Utc::now(),
            next_window_id: 1,
            status_flash: None,
        })
    }

    pub fn set_status_flash(&mut self, message: StatusMessage) {
        self.status_flash = Some(message);
    }

    pub fn take_status_flash(&mut self) -> Option<StatusMessage> {
        self.status_flash.take()
    }

    pub fn toggle_zoom(&mut self) {
        self.active_window().toggle_zoom();
    }

    pub fn enter_scroll_mode(&mut self) {
        self.active_window().enter_scroll_mode();
    }

    pub fn handle_scroll_input(&self, bytes: &[u8]) -> bool {
        self.windows[self.active_window].handle_scroll_input(bytes)
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    /// UTC timestamp when this session was created.
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Kill the child process backing a pane (used when terminating agents).
    pub fn kill_pane(&mut self, pane_id: usize) -> Result<()> {
        for window in &mut self.windows {
            if let Some(pane) = window.pane_mut(pane_id) {
                return pane.kill();
            }
        }
        anyhow::bail!("Pane {} not found in any window", pane_id)
    }

    pub fn active_window(&mut self) -> &mut Window {
        &mut self.windows[self.active_window]
    }

    /// Find which pane contains the given screen coordinates and set focus.
    /// Returns true if a pane was found and focused.
    pub fn focus_pane_at(&mut self, col: u16, row: u16) -> bool {
        let win = &self.windows[self.active_window];
        if let Some(pane_id) = win.pane_at(col, row) {
            self.windows[self.active_window].set_focus(pane_id);
            true
        } else {
            false
        }
    }

    /// Check if a position is on a pane border in the active window.
    pub fn border_at(&self, col: u16, row: u16) -> Option<(usize, usize, u16)> {
        self.windows[self.active_window].border_at(col, row)
    }

    /// Split the active window horizontally and launch a command in the new pane.
    /// Returns the new pane ID. Used by agent spawn to create agent panes.
    pub fn split_and_run_command(&mut self, command: &str) -> Result<usize> {
        let win = &mut self.windows[self.active_window];
        win.split_horizontal()?;
        let new_pane_id = win.focused_pane_id();

        // Send the command to the new pane's shell
        let cmd_bytes = format!("{}\n", command);
        win.send_input_to_pane(new_pane_id, cmd_bytes.as_bytes())?;

        Ok(new_pane_id)
    }

    /// Split horizontally and spawn a structured PTY command (no shell line injection).
    /// Returns `(pane_id, child_pid)`.
    pub fn split_and_spawn_agent(&mut self, spawn: AgentPtySpawn) -> Result<(usize, Option<u32>)> {
        let win = &mut self.windows[self.active_window];
        let pane_id = win.split_horizontal_with_command(spawn)?;
        let pid = win.pane_mut(pane_id).and_then(|p| p.process_id());
        Ok((pane_id, pid))
    }

    // ── Story 1.4: Window management ─────────────────────────────────────────

    /// Create a new window and switch focus to it.
    pub fn create_window(&mut self, rows: u16, cols: u16) -> Result<()> {
        let id = self.next_window_id;
        self.next_window_id += 1;
        let content_rows = rows.saturating_sub(1).max(1);
        let window = Window::new(id, content_rows, cols)?;
        self.windows.push(window);
        self.active_window = self.windows.len() - 1;
        Ok(())
    }

    /// Switch to window by 1-based index (Ctrl-b 1…9).
    pub fn switch_to_window(&mut self, one_based_idx: usize) {
        let idx = one_based_idx.saturating_sub(1);
        if idx < self.windows.len() {
            self.active_window = idx;
        }
    }

    pub fn next_window(&mut self) {
        if !self.windows.is_empty() {
            self.active_window = (self.active_window + 1) % self.windows.len();
        }
    }

    pub fn prev_window(&mut self) {
        if !self.windows.is_empty() {
            self.active_window =
                (self.active_window + self.windows.len() - 1) % self.windows.len();
        }
    }

    // ── Input forwarding ──────────────────────────────────────────────────────

    pub fn send_input(&mut self, bytes: &[u8]) -> Result<()> {
        self.active_window().send_input(bytes)
    }

    /// Send input to a specific pane by ID (searches all windows).
    /// Used by task send to deliver prompts to agent panes.
    pub fn send_input_to_pane(&mut self, pane_id: usize, bytes: &[u8]) -> Result<()> {
        for window in &mut self.windows {
            if window.pane_mut(pane_id).is_some() {
                return window.send_input_to_pane(pane_id, bytes);
            }
        }
        anyhow::bail!("Pane {} not found in any window", pane_id)
    }

    // ── Resize ────────────────────────────────────────────────────────────────

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        let content_rows = rows.saturating_sub(1).max(1);
        for w in &mut self.windows {
            w.resize(content_rows, cols)?;
        }
        Ok(())
    }

    /// Capture a serializable snapshot for the daemon protocol.
    pub fn snapshot(&self) -> SessionSnapshot {
        let status_message = self
            .status_flash
            .as_ref()
            .and_then(|m| m.text());
        SessionSnapshot {
            name: self.name.clone(),
            active_window: self.active_window,
            windows: self.windows.iter().map(|w| w.snapshot()).collect(),
            status_message,
        }
    }

    // ── Output polling ────────────────────────────────────────────────────────

    pub fn poll_output(&self) -> bool {
        self.windows.iter().any(|w| w.poll_output())
    }

    // ── Rendering ─────────────────────────────────────────────────────────────

    pub fn render(&mut self, frame: &mut Frame) {
        let total = frame.area();

        // Split vertically: content area + 1-row status bar
        let chunks = RatatuiLayout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(total);

        let content_area = chunks[0];
        let status_area = chunks[1];

        // Render active window
        let buf = frame.buffer_mut();
        self.windows[self.active_window].render(buf, content_area);

        // Render status bar
        let active_win = &self.windows[self.active_window];
        let status_state = StatusBarState {
            session_name: self.name.clone(),
            window_labels: self.windows.iter().map(|w| w.name.clone()).collect(),
            active_window_idx: self.active_window,
            in_scroll_mode: active_win.scroll_active(),
            zoom: active_win.zoom_state(),
            message: self
                .status_flash
                .clone()
                .unwrap_or(StatusMessage::None),
        };
        frame.render_widget(StatusBar::new(&status_state), status_area);
    }
}

// ── Session metadata (for daemon registry) ───────────────────────────────────

/// Lightweight record stored in the session registry file.
/// Used by `bmux ls` and `bmux attach/kill`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub name: String,
    pub pid: u32,
    pub window_count: usize,
    pub socket_path: PathBuf,
    pub created_at: DateTime<Utc>,
    /// Hex-encoded HMAC key for signing daemon protocol frames (0600 meta file).
    #[serde(default)]
    pub auth_key_hex: String,
}

impl SessionMeta {
    pub fn new(name: &str, window_count: usize, socket_path: PathBuf, auth_key_hex: String) -> Self {
        Self {
            name: name.to_string(),
            pid: std::process::id(),
            window_count,
            socket_path,
            created_at: Utc::now(),
            auth_key_hex,
        }
    }
}

/// Load the session auth key from registry metadata.
pub fn load_session_auth_key(session_name: &str) -> anyhow::Result<Vec<u8>> {
    let path = meta_path(session_name);
    let data = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("cannot read session meta {}: {e}", path.display()))?;
    let meta: SessionMeta = serde_json::from_str(&data)?;
    if meta.auth_key_hex.is_empty() {
        anyhow::bail!("session '{}' has no auth_key_hex in metadata", session_name);
    }
    crate::tui::session_auth::SessionAuth::from_hex(&meta.auth_key_hex)
        .map(|a| a.key_bytes().to_vec())
}

/// Path to the directory used as the session registry.
/// Each active session writes `<name>.json` here.
pub fn registry_dir() -> PathBuf {
    let dir = PathBuf::from("/tmp/bmux");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Path to the Unix socket for a named session.
pub fn socket_path(session_name: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/bmux-{}.sock", session_name))
}

/// Path to the metadata file for a named session.
pub fn meta_path(session_name: &str) -> PathBuf {
    registry_dir().join(format!("{}.json", session_name))
}

/// List all sessions visible in the registry.
pub fn list_sessions() -> Vec<SessionMeta> {
    let dir = registry_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut sessions = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json") {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(meta) = serde_json::from_str::<SessionMeta>(&data) {
                    // Verify socket still exists (session is alive)
                    if meta.socket_path.exists() {
                        sessions.push(meta);
                    } else {
                        // Stale entry – clean up
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
    }
    sessions
}
