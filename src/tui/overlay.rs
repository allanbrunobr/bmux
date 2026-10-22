//! Client-side prompts and overlays: window rename, task input and key help.
//!
//! Both the local TUI and the daemon client use these widgets. They capture
//! the keyboard while open, so keys are not forwarded to the focused pane.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use super::keybindings::HELP_ENTRIES;

/// What the text prompt is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptKind {
    RenameWindow,
    /// Send a task to the named agent.
    SendTask {
        agent: String,
    },
    /// Send a task and let the router choose the agent.
    SendTaskAuto,
}

impl PromptKind {
    fn label(&self) -> String {
        match self {
            PromptKind::RenameWindow => "Rename window:".to_string(),
            PromptKind::SendTask { agent } => format!("Task → {agent}:"),
            PromptKind::SendTaskAuto => "Task (auto-route):".to_string(),
        }
    }
}

/// Result of feeding one key to a prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptOutcome {
    /// Keep the prompt open.
    Pending,
    /// Esc or Ctrl-c: close without action.
    Cancelled,
    /// Enter with non-empty text.
    Submitted(String),
}

/// Single-line text prompt shown in the status bar row.
#[derive(Debug, Clone)]
pub struct Prompt {
    pub kind: PromptKind,
    input: String,
}

/// Longest text a prompt accepts (task prompts can be long).
const MAX_PROMPT_CHARS: usize = 4000;

impl Prompt {
    pub fn new(kind: PromptKind) -> Self {
        Self {
            kind,
            input: String::new(),
        }
    }

    /// Start with pre-filled text (e.g. the current window name).
    pub fn with_text(kind: PromptKind, text: &str) -> Self {
        Self {
            kind,
            input: text.to_string(),
        }
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> PromptOutcome {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => PromptOutcome::Cancelled,
            KeyCode::Char('c') if ctrl => PromptOutcome::Cancelled,
            KeyCode::Char('u') if ctrl => {
                self.input.clear();
                PromptOutcome::Pending
            }
            KeyCode::Enter => {
                let text = self.input.trim().to_string();
                if text.is_empty() {
                    PromptOutcome::Cancelled
                } else {
                    PromptOutcome::Submitted(text)
                }
            }
            KeyCode::Backspace => {
                self.input.pop();
                PromptOutcome::Pending
            }
            KeyCode::Char(c) if !ctrl => {
                if self.input.chars().count() < MAX_PROMPT_CHARS {
                    self.input.push(c);
                }
                PromptOutcome::Pending
            }
            _ => PromptOutcome::Pending,
        }
    }

    /// Draw the prompt over `area` (one row). Long input scrolls left so the
    /// cursor stays visible.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        let label = format!(" {} ", self.kind.label());
        let label_width = label.chars().count() as u16;
        let room = area.width.saturating_sub(label_width + 2) as usize;
        let chars: Vec<char> = self.input.chars().collect();
        let start = chars.len().saturating_sub(room);
        let visible: String = chars[start..].iter().collect();

        let line = Line::from(vec![
            Span::styled(
                label,
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::raw(visible),
            Span::styled(" ", Style::default().bg(Color::White)),
        ]);
        let row = Rect::new(area.x, area.y, area.width, 1);
        Clear.render(row, buf);
        Paragraph::new(line)
            .style(Style::default().bg(Color::Black).fg(Color::White))
            .render(row, buf);
    }
}

/// Draw the key binding help in the middle of `area`.
pub fn render_help(area: Rect, buf: &mut Buffer, prefix_label: &str) {
    let key_width = HELP_ENTRIES
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(0);
    let desc_width = HELP_ENTRIES
        .iter()
        .map(|(_, d)| d.chars().count())
        .max()
        .unwrap_or(0);
    let prefix_width = prefix_label.chars().count();

    let mut lines: Vec<Line> = HELP_ENTRIES
        .iter()
        .map(|(keys, desc)| {
            Line::from(vec![
                Span::styled(
                    format!(" {prefix_label} {keys:<key_width$} "),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(" {desc}")),
            ])
        })
        .collect();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!(" {prefix_label} {prefix_label}  sends {prefix_label} to the pane"),
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        " Press any key to close",
        Style::default().fg(Color::DarkGray),
    )));

    let width = (prefix_width + key_width + desc_width + 7) as u16;
    let height = lines.len() as u16 + 2;
    let w = width.min(area.width);
    let h = height.min(area.height);
    let popup = Rect::new(
        area.x + area.width.saturating_sub(w) / 2,
        area.y + area.height.saturating_sub(h) / 2,
        w,
        h,
    );
    Clear.render(popup, buf);
    Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" bmux keys "),
        )
        .render(popup, buf);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn typing_and_submit() {
        let mut p = Prompt::new(PromptKind::RenameWindow);
        for c in "api ".chars() {
            assert_eq!(p.handle_key(key(KeyCode::Char(c))), PromptOutcome::Pending);
        }
        p.handle_key(key(KeyCode::Char('x')));
        p.handle_key(key(KeyCode::Backspace));
        assert_eq!(
            p.handle_key(key(KeyCode::Enter)),
            PromptOutcome::Submitted("api".to_string())
        );
    }

    #[test]
    fn escape_and_empty_enter_cancel() {
        let mut p = Prompt::with_text(PromptKind::SendTaskAuto, "hello");
        assert_eq!(p.handle_key(key(KeyCode::Esc)), PromptOutcome::Cancelled);

        let mut p = Prompt::new(PromptKind::SendTaskAuto);
        assert_eq!(p.handle_key(key(KeyCode::Enter)), PromptOutcome::Cancelled);
    }

    #[test]
    fn ctrl_u_clears_and_ctrl_keys_are_not_typed() {
        let mut p = Prompt::with_text(PromptKind::RenameWindow, "old");
        p.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(p.input(), "");
        p.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
        assert_eq!(p.input(), "");
    }

    #[test]
    fn help_fits_in_a_small_area() {
        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);
        render_help(area, &mut buf, "Ctrl-b");
        let text: String = (0..area.width)
            .map(|x| buf[(x, 0)].symbol().to_string())
            .collect();
        assert!(text.contains("bmux keys"));
    }
}
