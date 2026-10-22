use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Actions that the app can perform in response to prefix-key sequences.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Action {
    // pane splitting
    SplitHorizontal,
    SplitVertical,
    // pane navigation
    FocusLeft,
    FocusDown,
    FocusUp,
    FocusRight,
    // pane resize
    ResizeLeft,
    ResizeDown,
    ResizeUp,
    ResizeRight,
    // window management
    NewWindow,
    NextWindow,
    PrevWindow,
    SwitchWindow(usize),
    // cycle focus (tmux 'o')
    FocusNextPane,
    // session management
    Detach,
    // pane zoom
    ToggleZoom,
    // scroll/copy mode
    EnterScrollMode,
    // config hot reload
    ReloadConfig,
    // Handled by the client: they open a prompt or an overlay.
    /// `Ctrl-b ,` — rename the active window.
    RenameWindow,
    /// `Ctrl-b t` — send a task to the agent in the focused pane.
    SendTask,
    /// `Ctrl-b T` — send a task and let BMUX pick the agent.
    SendTaskAuto,
    /// `Ctrl-b ?` — show the key binding help.
    ShowHelp,
}

/// A prefix key such as `Ctrl-b`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrefixKey {
    /// Lowercase ASCII letter, or `' '` for `ctrl-space`.
    ch: char,
}

impl PrefixKey {
    /// Parse `"ctrl-b"`, `"C-a"`, `"ctrl+space"` and similar spellings.
    pub fn parse(spec: &str) -> Option<Self> {
        let s = spec.trim().to_ascii_lowercase();
        let rest = s
            .strip_prefix("ctrl-")
            .or_else(|| s.strip_prefix("ctrl+"))
            .or_else(|| s.strip_prefix("c-"))?;
        match rest {
            "space" => Some(Self { ch: ' ' }),
            r if r.len() == 1 && r.as_bytes()[0].is_ascii_lowercase() => Some(Self {
                ch: r.as_bytes()[0] as char,
            }),
            _ => None,
        }
    }

    /// Human-readable label, e.g. `Ctrl-b`.
    pub fn label(&self) -> String {
        if self.ch == ' ' {
            "Ctrl-Space".to_string()
        } else {
            format!("Ctrl-{}", self.ch)
        }
    }

    fn matches(&self, event: &KeyEvent) -> bool {
        if !event.modifiers.contains(KeyModifiers::CONTROL) {
            return false;
        }
        match event.code {
            KeyCode::Char(c) => c.to_ascii_lowercase() == self.ch,
            _ => false,
        }
    }
}

impl Default for PrefixKey {
    fn default() -> Self {
        Self { ch: 'b' }
    }
}

/// Processes raw key events and implements the Ctrl-b prefix protocol.
///
/// - `Ctrl-b` activates "prefix mode"; the *next* key is interpreted as a command.
/// - Any key while prefix is active is consumed (not forwarded to the PTY).
/// - All other keys are passed through to the focused PTY unchanged.
pub struct KeybindingState {
    prefix_active: bool,
    prefix: PrefixKey,
}

impl KeybindingState {
    pub fn new() -> Self {
        Self::with_prefix(PrefixKey::default())
    }

    pub fn with_prefix(prefix: PrefixKey) -> Self {
        Self {
            prefix_active: false,
            prefix,
        }
    }

    /// Build from a config value; unknown spellings fall back to `Ctrl-b`.
    pub fn from_config(spec: &str) -> Self {
        let prefix = PrefixKey::parse(spec).unwrap_or_else(|| {
            tracing::warn!("Unknown prefix key '{spec}', using ctrl-b");
            PrefixKey::default()
        });
        Self::with_prefix(prefix)
    }

    pub fn prefix(&self) -> PrefixKey {
        self.prefix
    }

    pub fn is_prefix_active(&self) -> bool {
        self.prefix_active
    }

    /// Process one key event.
    ///
    /// Returns `(Option<Action>, bool)`:
    /// - `Some(action)` if a command was triggered
    /// - second element is `true` when the raw key bytes should be forwarded to the PTY
    pub fn process(&mut self, event: KeyEvent) -> (Option<Action>, bool) {
        if self.prefix_active {
            self.prefix_active = false;
            if self.prefix.matches(&event) {
                // Prefix pressed twice: send the prefix key itself to the PTY.
                return (None, true);
            }
            let action = Self::map_prefix_key(event.code);
            // Key is always consumed in prefix mode — never forwarded to PTY
            (action, false)
        } else if self.prefix.matches(&event) {
            self.prefix_active = true;
            // Ctrl-b itself is consumed; do not forward to PTY
            (None, false)
        } else {
            // Regular key — pass through to PTY
            (None, true)
        }
    }

    fn map_prefix_key(code: KeyCode) -> Option<Action> {
        match code {
            // Pane splitting
            KeyCode::Char('\\') => Some(Action::SplitHorizontal),
            KeyCode::Char('-') => Some(Action::SplitVertical),
            // Pane navigation lowercase
            KeyCode::Char('h') => Some(Action::FocusLeft),
            KeyCode::Char('j') => Some(Action::FocusDown),
            KeyCode::Char('k') => Some(Action::FocusUp),
            KeyCode::Char('l') => Some(Action::FocusRight),
            // Pane resize uppercase
            KeyCode::Char('H') => Some(Action::ResizeLeft),
            KeyCode::Char('J') => Some(Action::ResizeDown),
            KeyCode::Char('K') => Some(Action::ResizeUp),
            KeyCode::Char('L') => Some(Action::ResizeRight),
            // Window management
            KeyCode::Char('c') => Some(Action::NewWindow),
            KeyCode::Char('n') => Some(Action::NextWindow),
            KeyCode::Char('p') => Some(Action::PrevWindow),
            KeyCode::Char(c @ '1'..='9') => Some(Action::SwitchWindow((c as u8 - b'0') as usize)),
            // Focus cycle (tmux 'o')
            KeyCode::Char('o') => Some(Action::FocusNextPane),
            // Session management
            KeyCode::Char('d') => Some(Action::Detach),
            // Pane zoom
            KeyCode::Char('z') => Some(Action::ToggleZoom),
            // Scroll/copy mode
            KeyCode::Char('[') => Some(Action::EnterScrollMode),
            // Config hot reload
            KeyCode::Char('r') => Some(Action::ReloadConfig),
            KeyCode::Char(',') => Some(Action::RenameWindow),
            KeyCode::Char('t') => Some(Action::SendTask),
            KeyCode::Char('T') => Some(Action::SendTaskAuto),
            KeyCode::Char('?') => Some(Action::ShowHelp),
            // Unknown prefix sequence — silently consumed
            _ => None,
        }
    }
}

impl Default for KeybindingState {
    fn default() -> Self {
        Self::new()
    }
}

/// Key bindings shown by `Ctrl-b ?`, as (keys after the prefix, description).
pub const HELP_ENTRIES: &[(&str, &str)] = &[
    ("\\", "Split pane left/right"),
    ("-", "Split pane top/bottom"),
    ("h j k l", "Move focus between panes"),
    ("H J K L", "Resize the focused pane"),
    ("o", "Focus the next pane"),
    ("z", "Zoom the focused pane"),
    ("[", "Scroll mode (q or Esc to leave)"),
    ("c", "New window"),
    ("n / p", "Next / previous window"),
    ("1-9", "Go to window N"),
    (",", "Rename the current window"),
    ("t", "Send a task to the focused agent"),
    ("T", "Send a task (BMUX picks the agent)"),
    ("r", "Reload config.toml"),
    ("d", "Detach (the session keeps running)"),
    ("?", "Show this help"),
];

/// Convert a crossterm `KeyEvent` into the raw byte sequence a shell expects.
/// Handles printable ASCII, control characters, and common escape sequences.
pub fn key_event_to_bytes(event: KeyEvent) -> Vec<u8> {
    // Control characters: Ctrl+A = 0x01 … Ctrl+Z = 0x1A
    if event.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char(c) = event.code {
            let c = c.to_ascii_lowercase();
            if c.is_ascii_lowercase() {
                return vec![c as u8 - b'a' + 1];
            }
            if c == ' ' {
                return vec![0];
            }
        }
    }
    match event.code {
        KeyCode::Char(c) => {
            let mut buf = [0u8; 4];
            c.encode_utf8(&mut buf).as_bytes().to_vec()
        }
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Delete => vec![0x1b, b'[', b'3', b'~'],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => vec![0x1b, b'[', b'A'],
        KeyCode::Down => vec![0x1b, b'[', b'B'],
        KeyCode::Right => vec![0x1b, b'[', b'C'],
        KeyCode::Left => vec![0x1b, b'[', b'D'],
        KeyCode::Home => vec![0x1b, b'[', b'H'],
        KeyCode::End => vec![0x1b, b'[', b'F'],
        KeyCode::PageUp => vec![0x1b, b'[', b'5', b'~'],
        KeyCode::PageDown => vec![0x1b, b'[', b'6', b'~'],
        KeyCode::F(n) => match n {
            1 => vec![0x1b, b'O', b'P'],
            2 => vec![0x1b, b'O', b'Q'],
            3 => vec![0x1b, b'O', b'R'],
            4 => vec![0x1b, b'O', b'S'],
            5 => vec![0x1b, b'[', b'1', b'5', b'~'],
            6 => vec![0x1b, b'[', b'1', b'7', b'~'],
            7 => vec![0x1b, b'[', b'1', b'8', b'~'],
            8 => vec![0x1b, b'[', b'1', b'9', b'~'],
            _ => vec![],
        },
        _ => vec![],
    }
}
