/// Wire protocol between a bmux session daemon and its TUI clients.
///
/// All messages are newline-delimited JSON transported over a Unix socket.
use serde::{Deserialize, Serialize};

use super::keybindings::Action;

// ── Client → Server ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Initial handshake: tell the server the client's terminal size.
    Init { rows: u16, cols: u16 },
    /// The client's terminal was resized.
    Resize { rows: u16, cols: u16 },
    /// Raw keyboard bytes to forward to the focused PTY.
    Input { data: Vec<u8> },
    /// A prefix-key action (split, focus, window management, detach, …).
    Action { action: Action },
    /// Request a full state snapshot (used on initial connect).
    GetState,

    // ── CLI query messages (non-TUI clients) ──────────────────────────────
    /// List all agents in this session.
    AgentList,
    /// Get detailed status for one agent.
    AgentStatus { name: String },
    /// Spawn an agent.
    AgentSpawn { agent_type: String, name: String, model: Option<String> },
    /// Kill an agent.
    AgentKill { name: String },
    /// List all tasks.
    TaskList,
    /// Send a task to a specific agent.
    TaskSend { agent: Option<String>, content: String, model: Option<String> },
    /// Cancel a queued task.
    TaskCancel { id: String },
    /// Get task status.
    TaskStatus { id: String },
    /// Set a context key-value pair.
    ContextSet { key: String, value: String },
    /// Get a context value.
    ContextGet { key: String },
    /// List all context keys.
    ContextList,
    /// Dump context as JSON.
    ContextDump,
}

// ── Server → Client ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Full session state snapshot.
    State(SessionSnapshot),
    /// The session has been killed or the client was asked to detach.
    Detached,
    /// A non-fatal error message.
    Error { msg: String },

    // ── CLI query responses ───────────────────────────────────────────────
    /// JSON-encoded response to a CLI query.
    QueryResult { data: String },
}

// ── Snapshot types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub name: String,
    pub active_window: usize,
    pub windows: Vec<WindowSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSnapshot {
    pub id: usize,
    pub name: String,
    /// Ordered list of (pane_id, rect) computed from the BSP layout.
    pub pane_layouts: Vec<PaneLayout>,
    pub pane_cells: Vec<PaneSnapshot>,
    pub focused_pane: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneLayout {
    pub id: usize,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneSnapshot {
    pub id: usize,
    pub rows: u16,
    pub cols: u16,
    pub cells: Vec<Vec<CellData>>,
    pub cursor_row: u16,
    pub cursor_col: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellData {
    pub ch: char,
    pub fg_r: u8,
    pub fg_g: u8,
    pub fg_b: u8,
    pub bg_r: u8,
    pub bg_g: u8,
    pub bg_b: u8,
    pub bold: bool,
}

impl Default for CellData {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg_r: 0,
            fg_g: 0,
            fg_b: 0,
            bg_r: 0,
            bg_g: 0,
            bg_b: 0,
            bold: false,
        }
    }
}
