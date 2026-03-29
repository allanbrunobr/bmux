use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Actions that the app can perform in response to prefix-key sequences.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Action {
    // Story 1.2 – pane splitting
    SplitHorizontal,
    SplitVertical,
    // Story 1.3 – pane navigation
    FocusLeft,
    FocusDown,
    FocusUp,
    FocusRight,
    // Story 1.3 – pane resize
    ResizeLeft,
    ResizeDown,
    ResizeUp,
    ResizeRight,
    // Story 1.4 – window management
    NewWindow,
    NextWindow,
    PrevWindow,
    SwitchWindow(usize),
    // Story 1.3 – cycle focus (tmux 'o')
    FocusNextPane,
    // Story 1.5 – session management
    Detach,
    // Story 1.6 – pane zoom
    ToggleZoom,
    // Story 1.7 – scroll/copy mode
    EnterScrollMode,
    // Story 1.8 – config hot reload
    ReloadConfig,
}

/// Processes raw key events and implements the Ctrl-b prefix protocol.
///
/// - `Ctrl-b` activates "prefix mode"; the *next* key is interpreted as a command.
/// - Any key while prefix is active is consumed (not forwarded to the PTY).
/// - All other keys are passed through to the focused PTY unchanged.
pub struct KeybindingState {
    prefix_active: bool,
}

impl KeybindingState {
    pub fn new() -> Self {
        Self {
            prefix_active: false,
        }
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
            let action = Self::map_prefix_key(event.code);
            // Key is always consumed in prefix mode — never forwarded to PTY
            (action, false)
        } else if event.modifiers.contains(KeyModifiers::CONTROL)
            && event.code == KeyCode::Char('b')
        {
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
            // Pane splitting (Story 1.2)
            KeyCode::Char('\\') => Some(Action::SplitHorizontal),
            KeyCode::Char('-') => Some(Action::SplitVertical),
            // Pane navigation lowercase (Story 1.3)
            KeyCode::Char('h') => Some(Action::FocusLeft),
            KeyCode::Char('j') => Some(Action::FocusDown),
            KeyCode::Char('k') => Some(Action::FocusUp),
            KeyCode::Char('l') => Some(Action::FocusRight),
            // Pane resize uppercase (Story 1.3)
            KeyCode::Char('H') => Some(Action::ResizeLeft),
            KeyCode::Char('J') => Some(Action::ResizeDown),
            KeyCode::Char('K') => Some(Action::ResizeUp),
            KeyCode::Char('L') => Some(Action::ResizeRight),
            // Window management (Story 1.4)
            KeyCode::Char('c') => Some(Action::NewWindow),
            KeyCode::Char('n') => Some(Action::NextWindow),
            KeyCode::Char('p') => Some(Action::PrevWindow),
            KeyCode::Char(c @ '1'..='9') => {
                Some(Action::SwitchWindow((c as u8 - b'0') as usize))
            }
            // Focus cycle (tmux 'o')
            KeyCode::Char('o') => Some(Action::FocusNextPane),
            // Session management (Story 1.5)
            KeyCode::Char('d') => Some(Action::Detach),
            // Pane zoom (Story 1.6)
            KeyCode::Char('z') => Some(Action::ToggleZoom),
            // Scroll/copy mode (Story 1.7)
            KeyCode::Char('[') => Some(Action::EnterScrollMode),
            // Config hot reload (Story 1.8)
            KeyCode::Char('r') => Some(Action::ReloadConfig),
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
