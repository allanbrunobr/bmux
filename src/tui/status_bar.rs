use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use crate::tui::protocol::AgentBadge;
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
#[derive(Debug, Clone)]
pub struct StatusBarState {
    /// Current session name
    pub session_name: String,
    /// Window names in order; tabs are drawn as ` 1 name `.
    pub window_labels: Vec<String>,
    /// Index of the active window (0-based)
    pub active_window_idx: usize,
    /// Whether the active pane is in scroll mode
    pub in_scroll_mode: bool,
    /// Zoom state for the active window
    pub zoom: ZoomState,
    /// Transient status message
    pub message: StatusMessage,
    /// Agents in the session (daemon mode only).
    pub agents: Vec<AgentBadge>,
    /// The prefix key was pressed and the next key is a command.
    pub prefix_active: bool,
    /// Prefix key label for the help hint, e.g. `Ctrl-b`.
    pub prefix_label: String,
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
            agents: Vec::new(),
            prefix_active: false,
            prefix_label: "Ctrl-b".to_string(),
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
/// - Agents with their status and estimated cost, plus the session total (far right)
pub struct StatusBar<'a> {
    state: &'a StatusBarState,
}

impl<'a> StatusBar<'a> {
    pub fn new(state: &'a StatusBarState) -> Self {
        Self { state }
    }

    /// Build the left segment: ` session_name `
    fn left_spans(&self) -> Vec<Span<'static>> {
        vec![
            Span::styled(
                format!(" {} ", self.state.session_name),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]
    }

    /// Build the centre segment: window tabs ` 1 name `
    fn window_spans(&self) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        for (i, name) in self.state.window_labels.iter().enumerate() {
            let text = format!(" {} {name} ", i + 1);
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
        if self.state.prefix_active {
            spans.push(Span::styled(
                " PREFIX ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ));
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

        // Status message (config reload / error / flash)
        if let Some(msg) = self.state.message.text() {
            spans.push(Span::styled(format!(" {msg} "), message_style(&msg)));
        }

        spans
    }
}

/// Red for errors, blue otherwise.
fn message_style(msg: &str) -> Style {
    if msg.starts_with("Config error:") || msg.starts_with("Error") {
        Style::default().fg(Color::White).bg(Color::Red)
    } else {
        Style::default().fg(Color::Black).bg(Color::Blue)
    }
}

/// Per-agent badges: `● dev $0.0123`.
fn agent_spans(agents: &[AgentBadge]) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for agent in agents {
        let (indicator, color) = status_indicator(&agent.status);
        spans.push(Span::styled(
            format!(" {indicator}"),
            Style::default().fg(color).bg(Color::DarkGray),
        ));
        let mut text = format!(" {}", agent.name);
        if agent.cost_tracked {
            text.push(' ');
            text.push_str(&format_cost(agent.cost_usd));
        }
        text.push(' ');
        spans.push(Span::styled(
            text,
            Style::default().fg(Color::White).bg(Color::DarkGray),
        ));
    }
    spans
}

/// Session total for agents whose cost is tracked: `Σ $0.05`.
fn total_spans(agents: &[AgentBadge]) -> Vec<Span<'static>> {
    if !agents.iter().any(|a| a.cost_tracked) {
        return Vec::new();
    }
    let total: f64 = agents
        .iter()
        .filter(|a| a.cost_tracked)
        .map(|a| a.cost_usd)
        .sum();
    vec![Span::styled(
        format!(" Σ {} ", format_cost(total)),
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD),
    )]
}

/// Draw agent badges right-aligned on the status bar row. `left_width` is
/// the width already used on the left. The per-agent badges are dropped
/// first, then the total, when they do not fit.
pub fn render_agent_badges(agents: &[AgentBadge], left_width: usize, area: Rect, buf: &mut Buffer) {
    if agents.is_empty() || area.height == 0 {
        return;
    }
    let used = left_width + 1;
    let width = area.width as usize;
    let mut right = agent_spans(agents);
    right.extend(total_spans(agents));
    if used + spans_width(&right) > width {
        right = total_spans(agents);
    }
    let right_width = spans_width(&right);
    if right_width == 0 || used + right_width > width {
        return;
    }
    let x = area.left() + (width - right_width) as u16;
    buf.set_line(x, area.top(), &Line::from(right), right_width as u16);
}

/// Status dot and colour for an agent status label.
pub(crate) fn status_indicator(status: &str) -> (&'static str, Color) {
    match status {
        "working" => ("●", Color::LightGreen),
        "idle" => ("○", Color::Gray),
        "waiting" => ("◐", Color::Yellow),
        _ => ("✗", Color::LightRed),
    }
}

/// `$0.0042` below one cent, `$1.23` above.
pub fn format_cost(usd: f64) -> String {
    if usd > 0.0 && usd < 0.01 {
        format!("${usd:.4}")
    } else {
        format!("${usd:.2}")
    }
}

pub fn spans_width(spans: &[Span]) -> usize {
    spans.iter().map(|s| s.width()).sum()
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        // Clear the row (terminal default background).
        let row = Rect::new(area.x, area.y, area.width, 1);
        buf.set_style(row, Style::reset());
        for x in row.left()..row.right() {
            buf[(x, row.y)].set_symbol(" ");
        }

        let mut spans: Vec<Span> = Vec::new();
        spans.extend(self.left_spans());
        spans.extend(self.window_spans());
        spans.extend(self.right_spans());

        let left_width = spans_width(&spans);
        let line = Line::from(spans);
        buf.set_line(area.left(), area.top(), &line, area.width);
        render_agent_badges(&self.state.agents, left_width, area, buf);

        if self.state.agents.is_empty() && self.state.message == StatusMessage::None {
            let hint = format!("{} ? help ", self.state.prefix_label);
            let hint_width = Span::raw(hint.as_str()).width();
            let width = area.width as usize;
            if left_width + 1 + hint_width <= width {
                let x = area.left() + (width - hint_width) as u16;
                buf.set_string(x, area.top(), hint, Style::default().fg(Color::DarkGray));
            }
        }
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
        let row: String = (0..width)
            .map(|x| buffer[(x, 0)].symbol().to_string())
            .collect();
        row
    }

    #[test]
    fn renders_session_name() {
        let state = StatusBarState::new("my-project");
        let row = render_status_bar(&state, 80);
        assert!(
            row.contains("my-project"),
            "Expected session name in: {row:?}"
        );
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
        let state = StatusBarState {
            in_scroll_mode: true,
            ..Default::default()
        };
        let row = render_status_bar(&state, 80);
        assert!(row.contains("[SCROLL]"), "Expected [SCROLL] in: {row:?}");
    }

    #[test]
    fn no_scroll_indicator_when_not_in_scroll_mode() {
        let state = StatusBarState::default();
        let row = render_status_bar(&state, 80);
        assert!(
            !row.contains("[SCROLL]"),
            "Should not contain [SCROLL]: {row:?}"
        );
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
        assert!(
            !row.contains("[ZOOM]"),
            "Should not contain [ZOOM]: {row:?}"
        );
    }

    #[test]
    fn shows_config_reloaded_message() {
        let state = StatusBarState {
            message: StatusMessage::ConfigReloaded,
            ..Default::default()
        };
        let row = render_status_bar(&state, 80);
        assert!(
            row.contains("Config reloaded"),
            "Expected 'Config reloaded' in: {row:?}"
        );
    }

    #[test]
    fn shows_config_error_message() {
        let state = StatusBarState {
            message: StatusMessage::ConfigError("bad field".to_string()),
            ..Default::default()
        };
        let row = render_status_bar(&state, 80);
        assert!(
            row.contains("Config error"),
            "Expected 'Config error' in: {row:?}"
        );
        assert!(
            row.contains("bad field"),
            "Expected error detail in: {row:?}"
        );
    }

    #[test]
    fn no_message_when_none() {
        let state = StatusBarState::default();
        let row = render_status_bar(&state, 80);
        assert!(
            !row.contains("Config"),
            "Should not show config message: {row:?}"
        );
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

    fn badge(name: &str, status: &str, cost: f64, tracked: bool) -> AgentBadge {
        AgentBadge {
            name: name.to_string(),
            agent_type: "claude-code".to_string(),
            model: "claude-sonnet-4-5".to_string(),
            status: status.to_string(),
            tokens: 1000,
            cost_usd: cost,
            cost_tracked: tracked,
            pane_id: Some(1),
        }
    }

    #[test]
    fn shows_agent_cost_and_total() {
        let mut state = StatusBarState::new("demo");
        state.agents = vec![
            badge("dev", "working", 0.0042, true),
            badge("rev", "idle", 0.25, true),
            badge("sh", "idle", 0.0, false),
        ];
        let row = render_status_bar(&state, 100);
        assert!(row.contains("● dev $0.0042"), "row: {row}");
        assert!(row.contains("○ rev $0.25"), "row: {row}");
        assert!(row.contains("○ sh "), "row: {row}");
        assert!(row.contains("Σ $0.25"), "row: {row}");
    }

    #[test]
    fn narrow_bar_keeps_only_the_total() {
        let mut state = StatusBarState::new("demo");
        state.agents = vec![
            badge("developer", "working", 1.5, true),
            badge("reviewer", "idle", 0.5, true),
        ];
        let row = render_status_bar(&state, 36);
        assert!(row.contains("Σ $2.00"), "row: {row}");
        assert!(!row.contains("developer"), "row: {row}");
    }

    #[test]
    fn no_total_without_tracked_agents() {
        let mut state = StatusBarState::new("demo");
        state.agents = vec![badge("sh", "idle", 0.0, false)];
        let row = render_status_bar(&state, 80);
        assert!(!row.contains('Σ'), "row: {row}");
    }

    #[test]
    fn session_chip_has_no_brackets() {
        let state = StatusBarState::new("demo");
        let row = render_status_bar(&state, 60);
        assert!(row.starts_with(" demo "), "row: {row:?}");
        assert!(!row.contains("[demo]"), "row: {row:?}");
    }

    #[test]
    fn window_tabs_show_index_and_name() {
        let mut state = StatusBarState::new("demo");
        state.window_labels = vec!["parser".to_string(), "api".to_string()];
        let row = render_status_bar(&state, 60);
        assert!(row.contains(" demo   1 parser   2 api "), "row: {row:?}");
    }

    #[test]
    fn prefix_chip_only_when_prefix_active() {
        let mut state = StatusBarState::new("demo");
        let row = render_status_bar(&state, 60);
        assert!(!row.contains("PREFIX"), "row: {row:?}");
        state.prefix_active = true;
        let row = render_status_bar(&state, 60);
        assert!(row.contains(" PREFIX "), "row: {row:?}");
        let tab = row.find(" 1 1 ").unwrap();
        assert!(row.find("PREFIX").unwrap() > tab, "row: {row:?}");
    }

    #[test]
    fn help_hint_only_without_agents_and_message() {
        let mut state = StatusBarState::new("demo");
        state.prefix_label = "Ctrl-a".to_string();
        let row = render_status_bar(&state, 60);
        assert!(row.trim_end().ends_with("Ctrl-a ? help"), "row: {row:?}");

        state.message = StatusMessage::Custom("hello".to_string());
        let row = render_status_bar(&state, 60);
        assert!(!row.contains("? help"), "row: {row:?}");

        state.message = StatusMessage::None;
        state.agents = vec![badge("dev", "idle", 0.1, true)];
        let row = render_status_bar(&state, 60);
        assert!(!row.contains("? help"), "row: {row:?}");
    }

    #[test]
    fn error_messages_are_red() {
        assert_eq!(message_style("Error: boom").bg, Some(Color::Red));
        assert_eq!(message_style("Config error: x").bg, Some(Color::Red));
        assert_eq!(message_style("Task queued").bg, Some(Color::Blue));
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
