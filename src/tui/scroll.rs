/// Default maximum scrollback buffer size (lines).
pub const DEFAULT_HISTORY_LIMIT: usize = 10_000;

/// Visual indicator shown in the status bar when scroll mode is active.
pub const SCROLL_MODE_INDICATOR: &str = "[SCROLL]";

/// Tracks the scroll/copy mode state for a single pane.
///
/// When in scroll mode:
/// - A `[SCROLL]` indicator is shown.
/// - The user can navigate up/down through the scrollback buffer with k/j or arrow keys.
/// - Pressing `q` or `Escape` exits scroll mode and returns to live output.
///
/// The scrollback buffer is capped at `history_limit` lines.
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct ScrollState {
    /// Whether scroll mode is currently active
    active: bool,
    /// Index of the topmost visible line in the scrollback buffer.
    /// 0 = bottom (most recent), increases as the user scrolls up.
    offset: usize,
    /// Scrollback buffer: line 0 is oldest, last line is most recent.
    /// Uses VecDeque for O(1) eviction of oldest lines.
    buffer: VecDeque<String>,
    /// Maximum number of lines stored in the buffer.
    history_limit: usize,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::with_limit(DEFAULT_HISTORY_LIMIT)
    }
}

impl ScrollState {
    /// Create a new `ScrollState` with the default history limit.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new `ScrollState` with a custom history limit.
    pub fn with_limit(history_limit: usize) -> Self {
        Self {
            active: false,
            offset: 0,
            buffer: VecDeque::new(),
            history_limit,
        }
    }

    /// Returns `true` if scroll mode is currently active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Returns the current scroll offset (lines from the bottom).
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Returns the history limit for this pane.
    pub fn history_limit(&self) -> usize {
        self.history_limit
    }

    /// Returns the total number of lines in the scrollback buffer.
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    /// Returns the indicator string to display when scroll mode is active.
    /// Returns `None` when not in scroll mode.
    pub fn indicator(&self) -> Option<&str> {
        if self.active {
            Some(SCROLL_MODE_INDICATOR)
        } else {
            None
        }
    }

    /// Enter scroll mode. The view starts at the bottom (most recent output).
    pub fn enter(&mut self) {
        self.active = true;
        self.offset = 0;
    }

    /// Exit scroll mode and return to live output.
    pub fn exit(&mut self) {
        self.active = false;
        self.offset = 0;
    }

    /// Scroll up by `lines` (toward older content). Clamps at the top.
    ///
    /// Only has an effect when scroll mode is active.
    pub fn scroll_up(&mut self, lines: usize) {
        if !self.active {
            return;
        }
        let max_offset = self.buffer.len().saturating_sub(1);
        self.offset = (self.offset + lines).min(max_offset);
    }

    /// Scroll down by `lines` (toward newer content). Clamps at the bottom.
    ///
    /// Only has an effect when scroll mode is active.
    pub fn scroll_down(&mut self, lines: usize) {
        if !self.active {
            return;
        }
        self.offset = self.offset.saturating_sub(lines);
    }

    /// Append a line to the scrollback buffer.
    ///
    /// When the buffer exceeds `history_limit`, the oldest line is dropped.
    pub fn push_line(&mut self, line: impl Into<String>) {
        self.buffer.push_back(line.into());
        if self.buffer.len() > self.history_limit {
            self.buffer.pop_front();
        }
    }

    /// Append multiple lines at once (e.g., from PTY output).
    pub fn push_lines(&mut self, lines: impl IntoIterator<Item = impl Into<String>>) {
        for line in lines {
            self.push_line(line);
        }
    }

    /// Return a view into the buffer for rendering.
    ///
    /// `viewport_height` is the number of lines the pane can display.
    /// Returns lines from bottom-minus-offset, oldest first in the slice.
    pub fn visible_lines(&self, viewport_height: usize) -> Vec<&str> {
        if self.buffer.is_empty() {
            return vec![];
        }
        let total = self.buffer.len();
        let bottom = total.saturating_sub(self.offset);
        let top = bottom.saturating_sub(viewport_height);
        self.buffer.range(top..bottom).map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Construction ─────────────────────────────────────────────────────────

    #[test]
    fn new_state_is_inactive() {
        let s = ScrollState::new();
        assert!(!s.is_active());
        assert_eq!(s.offset(), 0);
        assert_eq!(s.buffer_len(), 0);
    }

    #[test]
    fn default_history_limit_is_10000() {
        let s = ScrollState::new();
        assert_eq!(s.history_limit(), DEFAULT_HISTORY_LIMIT);
        assert_eq!(s.history_limit(), 10_000);
    }

    #[test]
    fn custom_history_limit() {
        let s = ScrollState::with_limit(500);
        assert_eq!(s.history_limit(), 500);
    }

    // ── Enter / exit ─────────────────────────────────────────────────────────

    #[test]
    fn enter_activates_scroll_mode() {
        let mut s = ScrollState::new();
        s.enter();
        assert!(s.is_active());
    }

    #[test]
    fn indicator_shown_when_active() {
        let mut s = ScrollState::new();
        assert_eq!(s.indicator(), None);
        s.enter();
        assert_eq!(s.indicator(), Some(SCROLL_MODE_INDICATOR));
        assert_eq!(s.indicator(), Some("[SCROLL]"));
    }

    #[test]
    fn exit_deactivates_and_resets_offset() {
        let mut s = ScrollState::new();
        s.push_lines(["a", "b", "c", "d", "e"]);
        s.enter();
        s.scroll_up(3);
        assert_eq!(s.offset(), 3);
        s.exit();
        assert!(!s.is_active());
        assert_eq!(s.offset(), 0);
    }

    #[test]
    fn enter_starts_at_bottom() {
        let mut s = ScrollState::new();
        s.push_lines(["a", "b", "c"]);
        s.enter();
        assert_eq!(s.offset(), 0);
    }

    // ── Scrolling ────────────────────────────────────────────────────────────

    #[test]
    fn scroll_up_increases_offset() {
        let mut s = ScrollState::new();
        s.push_lines(["1", "2", "3", "4", "5"]);
        s.enter();
        s.scroll_up(2);
        assert_eq!(s.offset(), 2);
    }

    #[test]
    fn scroll_down_decreases_offset() {
        let mut s = ScrollState::new();
        s.push_lines(["1", "2", "3", "4", "5"]);
        s.enter();
        s.scroll_up(4);
        s.scroll_down(2);
        assert_eq!(s.offset(), 2);
    }

    #[test]
    fn scroll_up_clamps_at_top() {
        let mut s = ScrollState::new();
        s.push_lines(["a", "b", "c"]);
        s.enter();
        s.scroll_up(1000); // way beyond the top
        assert_eq!(s.offset(), 2); // max is buffer_len - 1 = 2
    }

    #[test]
    fn scroll_down_clamps_at_bottom() {
        let mut s = ScrollState::new();
        s.push_lines(["a", "b", "c"]);
        s.enter();
        s.scroll_up(2);
        s.scroll_down(1000); // way beyond the bottom
        assert_eq!(s.offset(), 0);
    }

    #[test]
    fn scroll_has_no_effect_when_inactive() {
        let mut s = ScrollState::new();
        s.push_lines(["a", "b", "c"]);
        // NOT calling s.enter()
        s.scroll_up(2);
        assert_eq!(s.offset(), 0);
        s.scroll_down(1);
        assert_eq!(s.offset(), 0);
    }

    // ── Buffer management ────────────────────────────────────────────────────

    #[test]
    fn push_line_appends_to_buffer() {
        let mut s = ScrollState::new();
        s.push_line("hello");
        assert_eq!(s.buffer_len(), 1);
    }

    #[test]
    fn buffer_capped_at_history_limit() {
        let limit = 5;
        let mut s = ScrollState::with_limit(limit);
        for i in 0..20 {
            s.push_line(format!("line {i}"));
        }
        assert_eq!(s.buffer_len(), limit);
    }

    #[test]
    fn buffer_keeps_newest_lines_when_capped() {
        let mut s = ScrollState::with_limit(3);
        s.push_lines(["a", "b", "c", "d", "e"]);
        // Should keep c, d, e
        let visible = s.visible_lines(3);
        assert_eq!(visible, &["c", "d", "e"]);
    }

    #[test]
    fn history_limit_at_least_10000() {
        // AC: buffer supports at least 10,000 lines
        let mut s = ScrollState::new();
        assert!(s.history_limit() >= 10_000);
        for i in 0..10_000 {
            s.push_line(format!("line {i}"));
        }
        assert_eq!(s.buffer_len(), 10_000);
    }

    // ── Visible lines ────────────────────────────────────────────────────────

    #[test]
    fn visible_lines_empty_buffer() {
        let s = ScrollState::new();
        assert!(s.visible_lines(10).is_empty());
    }

    #[test]
    fn visible_lines_at_bottom() {
        let mut s = ScrollState::new();
        s.push_lines(["a", "b", "c", "d", "e"]);
        s.enter();
        // viewport shows last 3 lines
        let v = s.visible_lines(3);
        assert_eq!(v, &["c", "d", "e"]);
    }

    #[test]
    fn visible_lines_scrolled_up() {
        let mut s = ScrollState::new();
        s.push_lines(["a", "b", "c", "d", "e"]);
        s.enter();
        s.scroll_up(2);
        // offset=2, bottom=3, top=0 (3-3=0) → lines 0..3 = a,b,c
        let v = s.visible_lines(3);
        assert_eq!(v, &["a", "b", "c"]);
    }

    // ── q / Escape exits scroll mode (AC) ────────────────────────────────────

    #[test]
    fn q_or_escape_exits_scroll_mode() {
        // The ScrollState.exit() method models q/Escape behavior
        let mut s = ScrollState::new();
        s.push_line("content");
        s.enter();
        assert!(s.is_active());
        // Simulate pressing q or Escape
        s.exit();
        assert!(!s.is_active());
    }

    // ── Ctrl-b [ pattern (AC) ────────────────────────────────────────────────

    #[test]
    fn ctrl_b_bracket_enters_scroll_and_shows_indicator() {
        let mut s = ScrollState::new();
        s.push_lines(["line1", "line2", "line3"]);
        // Simulate Ctrl-b [
        s.enter();
        assert!(s.is_active());
        assert_eq!(s.indicator(), Some("[SCROLL]"));
    }

    #[test]
    fn j_key_scrolls_down_k_key_scrolls_up() {
        let mut s = ScrollState::new();
        s.push_lines((0..20).map(|i| format!("line {i}")));
        s.enter();
        // k = scroll up
        s.scroll_up(5);
        assert_eq!(s.offset(), 5);
        // j = scroll down
        s.scroll_down(3);
        assert_eq!(s.offset(), 2);
    }
}
