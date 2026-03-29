use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use crate::tui::scroll::SCROLL_MODE_INDICATOR;
use crate::tui::zoom::ZoomState;

/// Transient status message displayed in the status bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusMessage {
    /// "Config reloaded" — shown after a successful hot reload
    ConfigReloaded,
    /// "Config error: {details}" — shown after a failed hot reload
    ConfigError(String),
    /// Custom / arbitrary message
    Custom(String),
    /// No message
    None,
}

impl StatusMessage {
    pub fn text(&self) -> Option<String> {
        match self {
            StatusMessage::ConfigReloaded => Some("Config reloaded".to_string()),
            StatusMessage::ConfigError(e) => Some(format!("Config error: {e}")),
            StatusMessage::Custom(s) => Some(s.clone()),
            StatusMessage::None => None,
        }
    }
}

/// State passed to `StatusBar` for rendering.
///
/// `wt3` will extend this with agent info at merge time.
#[derive(Debug, Clone)]
pub struct StatusBarState {
    /// Current session name
    pub session_name: String,
    /// Window index list, e.g. `[1*, 2, 3]`
    pub window_labels: Vec<String>,
    /// Index of the active window (0-based)
    pub active_window_idx: usize,
    /// Whether the active pane is in scroll mode
    pub in_scroll_mode: bool,
    /// Zoom state for the active window
    pub zoom: ZoomState,
    /// Transient status message
    pub message: StatusMessage,
}

impl Default for StatusBarState {
    fn default() -> Self {
        Self {
            session_name: "bmux".to_string(),
            window_labels: vec!["1".to_string()],
            active_window_idx: 0,
            in_scroll_mode: false,
            zoom: ZoomState::new(),
            message: StatusMessage::None,
        }
    }
}

impl StatusBarState {
    pub fn new(session_name: impl Into<String>) -> Self {
        Self {
            session_name: session_name.into(),
            ..Default::default()
        }
    }
}

/// Ratatui widget for the BMUX status bar.
///
/// Renders a single line at the bottom of the terminal with:
/// - Session name (left)
/// - Window list (centre)
/// - Zoom / scroll indicators and status messages (right)
///
/// `wt3` extends this with per-agent status and cost info.
pub struct StatusBar<'a> {
    state: &'a StatusBarState,
}

impl<'a> StatusBar<'a> {
    pub fn new(state: &'a StatusBarState) -> Self {
        Self { state }
    }

    /// Build the left segment: `[session_name]`
    fn left_spans(&self) -> Vec<Span<'static>> {
        vec![
            Span::styled(
                format!(" [{}] ", self.state.session_name.clone()),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]
    }

    /// Build the centre segment: window list
    fn window_spans(&self) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        for (i, label) in self.state.window_labels.iter().enumerate() {
            let text = format!(" {label} ");
            if i == self.state.active_window_idx {
                spans.push(Span::styled(
                    text,
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::styled(
                    text,
                    Style::default().fg(Color::White).bg(Color::DarkGray),
                ));
            }
            spans.push(Span::raw(" "));
        }
        spans
    }

    /// Build the right segment: indicators and messages
    fn right_spans(&self) -> Vec<Span<'static>> {
        let mut spans = Vec::new();

        // Scroll mode indicator
        if self.state.in_scroll_mode {
            spans.push(Span::styled(
                format!(" {SCROLL_MODE_INDICATOR} "),
                Style::default().fg(Color::Black).bg(Color::Cyan),
            ));
        }

        // Zoom indicator
        if self.state.zoom.is_zoomed() {
            spans.push(Span::styled(
                " [ZOOM] ",
                Style::default().fg(Color::Black).bg(Color::Magenta),
            ));
        }

        // Status message (config reload / error)
        if let Some(msg) = self.state.message.text() {
            let style = match &self.state.message {
                StatusMessage::ConfigError(_) => {
                    Style::default().fg(Color::White).bg(Color::Red)
                }
                _ => Style::default().fg(Color::Black).bg(Color::Blue),
            };
            spans.push(Span::styled(format!(" {msg} "), style));
        }

        spans
    }
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Fill the background
        for x in area.left()..area.right() {
            buf[(x, area.top())]
                .set_style(Style::default().fg(Color::White).bg(Color::DarkGray));
        }

        let mut spans: Vec<Span> = Vec::new();
        spans.extend(self.left_spans());
        spans.extend(self.window_spans());
        spans.extend(self.right_spans());

        let line = Line::from(spans);
        buf.set_line(area.left(), area.top(), &line, area.width);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn render_status_bar(state: &StatusBarState, width: u16) -> String {
        let backend = TestBackend::new(width, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                frame.render_widget(StatusBar::new(state), area);
            })
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        let row: String = (0..width).map(|x| buffer[(x, 0)].symbol().to_string()).collect();
        row
    }

    #[test]
    fn renders_session_name() {
        let state = StatusBarState::new("my-project");
        let row = render_status_bar(&state, 80);
        assert!(row.contains("my-project"), "Expected session name in: {row:?}");
    }

    #[test]
    fn renders_window_label() {
        let mut state = StatusBarState::new("s");
        state.window_labels = vec!["1".to_string(), "2".to_string()];
        state.active_window_idx = 0;
        let row = render_status_bar(&state, 80);
        assert!(row.contains('1'), "Window labels should appear: {row:?}");
    }

    #[test]
    fn shows_scroll_indicator_when_in_scroll_mode() {
        let mut state = StatusBarState::default();
        state.in_scroll_mode = true;
        let row = render_status_bar(&state, 80);
        assert!(row.contains("[SCROLL]"), "Expected [SCROLL] in: {row:?}");
    }

    #[test]
    fn no_scroll_indicator_when_not_in_scroll_mode() {
        let state = StatusBarState::default();
        let row = render_status_bar(&state, 80);
        assert!(!row.contains("[SCROLL]"), "Should not contain [SCROLL]: {row:?}");
    }

    #[test]
    fn shows_zoom_indicator_when_zoomed() {
        let mut state = StatusBarState::default();
        state.zoom.toggle(1);
        let row = render_status_bar(&state, 80);
        assert!(row.contains("[ZOOM]"), "Expected [ZOOM] in: {row:?}");
    }

    #[test]
    fn no_zoom_indicator_when_not_zoomed() {
        let state = StatusBarState::default();
        let row = render_status_bar(&state, 80);
        assert!(!row.contains("[ZOOM]"), "Should not contain [ZOOM]: {row:?}");
    }

    #[test]
    fn shows_config_reloaded_message() {
        let mut state = StatusBarState::default();
        state.message = StatusMessage::ConfigReloaded;
        let row = render_status_bar(&state, 80);
        assert!(
            row.contains("Config reloaded"),
            "Expected 'Config reloaded' in: {row:?}"
        );
    }

    #[test]
    fn shows_config_error_message() {
        let mut state = StatusBarState::default();
        state.message = StatusMessage::ConfigError("bad field".to_string());
        let row = render_status_bar(&state, 80);
        assert!(
            row.contains("Config error"),
            "Expected 'Config error' in: {row:?}"
        );
        assert!(row.contains("bad field"), "Expected error detail in: {row:?}");
    }

    #[test]
    fn no_message_when_none() {
        let state = StatusBarState::default();
        let row = render_status_bar(&state, 80);
        assert!(!row.contains("Config"), "Should not show config message: {row:?}");
    }

    // ── StatusMessage unit tests ─────────────────────────────────────────────

    #[test]
    fn status_message_config_reloaded_text() {
        let m = StatusMessage::ConfigReloaded;
        assert_eq!(m.text(), Some("Config reloaded".to_string()));
    }

    #[test]
    fn status_message_config_error_includes_details() {
        let m = StatusMessage::ConfigError("invalid key".to_string());
        let t = m.text().unwrap();
        assert!(t.starts_with("Config error:"));
        assert!(t.contains("invalid key"));
    }

    #[test]
    fn status_message_none_returns_none() {
        assert_eq!(StatusMessage::None.text(), None);
    }
}

/// Row of agent status data for display
#[derive(Debug, Clone)]
pub struct AgentStatusRow {
    pub icon: String,
    pub name: String,
    pub status_indicator: String,
    pub model: String,
    pub cost_usd: f64,
}

/// Aggregate statistics across all agents
#[derive(Debug, Clone, Default)]
pub struct AggregateStats {
    pub total_agents: usize,
    pub active_agents: usize,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub session_seconds: u64,
}

/// Trait for providing agent status data to the status bar
pub trait AgentStatusProvider {
    fn agent_status_rows(&self) -> Vec<AgentStatusRow>;
    fn aggregate_stats(&self) -> AggregateStats;
}
