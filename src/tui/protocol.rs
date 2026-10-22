/// Wire protocol between a bmux session daemon and its TUI clients.
///
/// All messages are newline-delimited JSON transported over a Unix socket.
use serde::{Deserialize, Serialize};

use super::keybindings::Action;

// ── Client → Server ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Mouse click at (col, row) — server handles focus change.
    MouseClick { col: u16, row: u16 },
    /// Mouse drag at (col, row) with left pane ID for border resize.
    MouseDrag { col: u16, row: u16 },
    /// Mouse button released — end drag.
    MouseUp,
    /// Rename the active window (`Ctrl-b ,`).
    RenameWindow { name: String },

    // ── CLI query messages (non-TUI clients) ──────────────────────────────
    /// List all agents in this session.
    AgentList,
    /// Get detailed status for one agent.
    AgentStatus { name: String },
    /// Spawn an agent.
    AgentSpawn {
        agent_type: String,
        name: String,
        model: Option<String>,
    },
    /// Kill an agent.
    AgentKill { name: String },
    /// List all tasks.
    TaskList,
    /// Send a task to a specific agent.
    TaskSend {
        agent: Option<String>,
        content: String,
        model: Option<String>,
    },
    /// Cancel a queued task.
    TaskCancel { id: String },
    /// Get task status.
    TaskStatus { id: String },
    /// Set a context key-value pair.
    ContextSet {
        key: String,
        value: String,
        /// Optional writer identity (defaults to "cli" in the daemon).
        #[serde(default)]
        author: Option<String>,
    },
    /// Get a context value.
    ContextGet { key: String },
    /// List all context keys.
    ContextList,
    /// Dump context as JSON.
    ContextDump,
    /// Run a workflow YAML file in the background.
    WorkflowRun { file: String, input: Option<String> },
    /// Get workflow run status by ID.
    WorkflowStatus { id: String },
    /// Re-read config.toml in the running session (`bmux config reload`).
    ConfigReload,
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
    #[serde(default)]
    pub status_message: Option<String>,
    /// Agents running in this session, shown on the right of the status bar.
    #[serde(default)]
    pub agents: Vec<AgentBadge>,
    /// `false` hides the status bar (`[tui] status_bar = false`).
    #[serde(default = "default_true")]
    pub status_bar: bool,
}

fn default_true() -> bool {
    true
}

/// Compact agent state for the status bar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentBadge {
    pub name: String,
    pub agent_type: String,
    pub model: String,
    /// `idle`, `working`, `waiting` or `error: …`.
    pub status: String,
    pub tokens: u64,
    pub cost_usd: f64,
    /// `false` when the agent type does not report usage.
    pub cost_tracked: bool,
    pub pane_id: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSnapshot {
    pub id: usize,
    pub name: String,
    /// Ordered list of (pane_id, rect) computed from the BSP layout.
    pub pane_layouts: Vec<PaneLayout>,
    pub pane_cells: Vec<PaneSnapshot>,
    pub focused_pane: usize,
    #[serde(default)]
    pub zoomed: bool,
    #[serde(default)]
    pub scroll_active: bool,
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
    /// `false` when the application hid the cursor (`ESC [ ? 25 l`) or the
    /// pane is in scroll mode.
    #[serde(default = "default_true")]
    pub cursor_visible: bool,
}

/// Terminal colour as reported by the emulator.
///
/// Indexed colours are kept as indices so the client's terminal theme
/// decides how they look (instead of a fixed RGB approximation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CellColor {
    /// The terminal's default foreground/background.
    #[default]
    Default,
    /// Palette index 0–255.
    Idx(u8),
    /// 24-bit colour.
    Rgb(u8, u8, u8),
}

impl CellColor {
    fn is_default(&self) -> bool {
        *self == CellColor::Default
    }
}

/// One terminal cell. Default values are not serialized, so blank cells
/// encode as `{}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellData {
    /// Cell contents (a full grapheme); `" "` for an empty cell.
    #[serde(default = "blank_cell", skip_serializing_if = "is_blank_cell")]
    pub ch: String,
    #[serde(default, skip_serializing_if = "CellColor::is_default")]
    pub fg: CellColor,
    #[serde(default, skip_serializing_if = "CellColor::is_default")]
    pub bg: CellColor,
    #[serde(default, skip_serializing_if = "is_false")]
    pub bold: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub italic: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub underline: bool,
    /// Reverse video (swap foreground and background).
    #[serde(default, skip_serializing_if = "is_false")]
    pub inverse: bool,
    /// Second cell of a wide (double-width) character; draws nothing.
    #[serde(default, skip_serializing_if = "is_false")]
    pub wide_cont: bool,
}

fn blank_cell() -> String {
    " ".to_string()
}

fn is_blank_cell(ch: &str) -> bool {
    ch == " "
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl Default for CellData {
    fn default() -> Self {
        Self {
            ch: blank_cell(),
            fg: CellColor::Default,
            bg: CellColor::Default,
            bold: false,
            italic: false,
            underline: false,
            inverse: false,
            wide_cont: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cell_serializes_empty_and_round_trips() {
        let cell = CellData::default();
        let json = serde_json::to_string(&cell).unwrap();
        assert_eq!(json, "{}");
        let back: CellData = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cell);
    }

    #[test]
    fn styled_cell_round_trips() {
        let cell = CellData {
            ch: "界".to_string(),
            fg: CellColor::Idx(196),
            bg: CellColor::Rgb(1, 2, 3),
            bold: true,
            italic: true,
            underline: true,
            inverse: true,
            wide_cont: false,
        };
        let json = serde_json::to_string(&cell).unwrap();
        assert!(!json.contains("wide_cont"), "{json}");
        let back: CellData = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cell);
    }

    #[test]
    fn cell_without_new_fields_uses_defaults() {
        let back: CellData = serde_json::from_str(r#"{"ch":"a","bold":true}"#).unwrap();
        assert_eq!(back.ch, "a");
        assert!(back.bold);
        assert_eq!(back.fg, CellColor::Default);
        assert_eq!(back.bg, CellColor::Default);
        assert!(!back.italic && !back.underline && !back.inverse && !back.wide_cont);
    }

    #[test]
    fn pane_snapshot_without_cursor_visible_defaults_to_visible() {
        let json = r#"{"id":1,"rows":1,"cols":1,"cells":[[{}]],"cursor_row":0,"cursor_col":0}"#;
        let snap: PaneSnapshot = serde_json::from_str(json).unwrap();
        assert!(snap.cursor_visible);
        assert_eq!(snap.cells[0][0], CellData::default());
    }
}
