//! Shared pane drawing: cell styles, pane borders and agent pane titles.
//!
//! Used by both the daemon client (`client.rs`) and the local TUI
//! (`pane.rs`), so the two modes render pane content the same way.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders},
};

use super::{
    protocol::{AgentBadge, CellColor, CellData, PaneSnapshot},
    status_bar::{format_cost, status_indicator},
};

/// Panes narrower than this get no title.
const MIN_TITLE_PANE_WIDTH: u16 = 12;

/// Map a snapshot colour to a ratatui colour.
pub fn cell_color(c: CellColor) -> Color {
    match c {
        CellColor::Default => Color::Reset,
        CellColor::Idx(i) => Color::Indexed(i),
        CellColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

/// Style for one terminal cell (colours and attributes).
pub fn cell_style(cell: &CellData) -> Style {
    let mut style = Style::default()
        .fg(cell_color(cell.fg))
        .bg(cell_color(cell.bg));
    if cell.bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    if cell.italic {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if cell.underline {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    if cell.inverse {
        style = style.add_modifier(Modifier::REVERSED);
    }
    style
}

/// Draw the cells of a pane snapshot into `inner` (the area inside the
/// border). Wide-continuation cells are skipped so the wide character in
/// the previous cell keeps both columns.
pub fn render_pane_cells(buf: &mut Buffer, inner: Rect, snap: &PaneSnapshot) {
    let inner = inner.intersection(buf.area);
    for (row_idx, row) in snap.cells.iter().enumerate() {
        let y = inner.y + row_idx as u16;
        if row_idx >= inner.height as usize {
            break;
        }
        for (col_idx, cell) in row.iter().enumerate() {
            if col_idx >= inner.width as usize {
                break;
            }
            if cell.wide_cont {
                continue;
            }
            let x = inner.x + col_idx as u16;
            let style = cell_style(cell);
            // A wide character whose second half is clipped by the pane
            // edge would spill over the border: draw a blank instead.
            let clipped = row.get(col_idx + 1).is_some_and(|next| next.wide_cont)
                && col_idx + 1 >= inner.width as usize;
            let text = if cell.ch.is_empty() || clipped {
                " "
            } else {
                cell.ch.as_str()
            };
            buf[(x, y)].set_symbol(text).set_style(style);
        }
    }
}

/// Terminal cursor position for a pane snapshot drawn into `inner`, or
/// `None` when the cursor is hidden or outside the visible area.
pub fn cursor_position(inner: Rect, snap: &PaneSnapshot) -> Option<(u16, u16)> {
    if !snap.cursor_visible || snap.cursor_row >= snap.rows || snap.cols == 0 {
        return None;
    }
    // After a write to the last column, vt100 leaves the cursor one past
    // the right margin (pending wrap). Show it on the last column.
    let col = snap.cursor_col.min(snap.cols - 1);
    if snap.cursor_row >= inner.height || col >= inner.width {
        return None;
    }
    Some((inner.x + col, inner.y + snap.cursor_row))
}

/// Rounded pane border: Yellow when focused, DarkGray otherwise.
pub fn pane_block(focused: bool) -> Block<'static> {
    let color = if focused {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color))
}

/// `claude-sonnet-4-5` → `sonnet-4-5`.
fn short_model(model: &str) -> &str {
    model.strip_prefix("claude-").unwrap_or(model)
}

fn str_width(s: &str) -> usize {
    Span::raw(s).width()
}

/// Title for the top border of an agent pane: ` ● dev · sonnet-4-5 · $0.14 `.
///
/// The status symbol uses the same colour as the status-bar badge; the
/// name is bold (Yellow when the pane is focused). The title is cut with
/// `…` to fit `pane_width - 4` columns, and omitted for very narrow panes.
pub fn pane_title(agent: &AgentBadge, focused: bool, pane_width: u16) -> Option<Line<'static>> {
    if pane_width < MIN_TITLE_PANE_WIDTH {
        return None;
    }
    let max = (pane_width - 4) as usize;
    let (symbol, symbol_color) = status_indicator(&agent.status);
    let name_style = Style::default()
        .fg(if focused { Color::Yellow } else { Color::White })
        .add_modifier(Modifier::BOLD);
    let meta_style = Style::default().fg(Color::Gray);

    let mut meta = String::new();
    let model = short_model(&agent.model);
    if !model.is_empty() {
        meta.push_str(" · ");
        meta.push_str(model);
    }
    if agent.cost_tracked {
        meta.push_str(" · ");
        meta.push_str(&format_cost(agent.cost_usd));
    }
    meta.push(' ');

    let parts = vec![
        (" ".to_string(), Style::default()),
        (symbol.to_string(), Style::default().fg(symbol_color)),
        (" ".to_string(), Style::default()),
        (agent.name.clone(), name_style),
        (meta, meta_style),
    ];
    Some(Line::from(truncate_parts(parts, max)))
}

/// Keep styled parts within `max` columns; when cut, the last visible
/// character is replaced by `…`.
fn truncate_parts(parts: Vec<(String, Style)>, max: usize) -> Vec<Span<'static>> {
    let total: usize = parts.iter().map(|(t, _)| str_width(t)).sum();
    if total <= max {
        return parts.into_iter().map(|(t, s)| Span::styled(t, s)).collect();
    }
    let budget = max.saturating_sub(1); // room for the ellipsis
    let mut used = 0;
    let mut spans = Vec::new();
    let mut last_style = Style::default();
    'outer: for (text, style) in parts {
        let mut kept = String::new();
        for ch in text.chars() {
            let w = str_width(ch.encode_utf8(&mut [0; 4]));
            if used + w > budget {
                if !kept.is_empty() {
                    spans.push(Span::styled(kept, style));
                }
                last_style = style;
                break 'outer;
            }
            used += w;
            kept.push(ch);
        }
        last_style = style;
        if !kept.is_empty() {
            spans.push(Span::styled(kept, style));
        }
    }
    spans.push(Span::styled("…", last_style));
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(model: &str, cost: f64, tracked: bool) -> AgentBadge {
        AgentBadge {
            name: "dev".to_string(),
            agent_type: "claude-code".to_string(),
            model: model.to_string(),
            status: "working".to_string(),
            tokens: 0,
            cost_usd: cost,
            cost_tracked: tracked,
            pane_id: Some(1),
        }
    }

    fn text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn full_title_strips_claude_prefix() {
        let line = pane_title(&agent("claude-sonnet-4-5", 0.14, true), true, 80).unwrap();
        assert_eq!(text(&line), " ● dev · sonnet-4-5 · $0.14 ");
        let name = line.spans.iter().find(|s| s.content == "dev").unwrap();
        assert_eq!(name.style.fg, Some(Color::Yellow));
        assert!(name.style.add_modifier.contains(Modifier::BOLD));
        let dot = line.spans.iter().find(|s| s.content == "●").unwrap();
        assert_eq!(dot.style.fg, Some(Color::LightGreen));
    }

    #[test]
    fn unfocused_name_is_not_yellow() {
        let line = pane_title(&agent("gpt-5", 0.0, false), false, 80).unwrap();
        let name = line.spans.iter().find(|s| s.content == "dev").unwrap();
        assert_ne!(name.style.fg, Some(Color::Yellow));
        assert_eq!(text(&line), " ● dev · gpt-5 ");
    }

    #[test]
    fn untracked_cost_and_empty_model_are_omitted() {
        let line = pane_title(&agent("", 1.0, false), true, 80).unwrap();
        assert_eq!(text(&line), " ● dev ");
    }

    #[test]
    fn long_title_is_truncated_with_ellipsis() {
        let line = pane_title(&agent("claude-sonnet-4-5", 0.14, true), true, 16).unwrap();
        let t = text(&line);
        assert!(t.ends_with('…'), "{t:?}");
        assert_eq!(str_width(&t), 12, "{t:?}");
        assert!(t.starts_with(" ● dev"), "{t:?}");
    }

    #[test]
    fn tiny_pane_has_no_title() {
        assert!(pane_title(&agent("claude-x", 0.1, true), true, 11).is_none());
    }

    #[test]
    fn wide_cells_render_and_continuations_are_skipped() {
        let mut cells = vec![CellData::default(); 3];
        cells[0].ch = "界".to_string();
        cells[1].wide_cont = true;
        cells[2].ch = "x".to_string();
        cells[2].inverse = true;
        let snap = PaneSnapshot {
            id: 0,
            rows: 1,
            cols: 3,
            cells: vec![cells],
            cursor_row: 0,
            cursor_col: 0,
            cursor_visible: true,
        };
        let area = Rect::new(0, 0, 3, 1);
        let mut buf = Buffer::empty(area);
        render_pane_cells(&mut buf, area, &snap);
        assert_eq!(buf[(0, 0)].symbol(), "界");
        assert_eq!(buf[(2, 0)].symbol(), "x");
        assert!(buf[(2, 0)].modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn hidden_cursor_has_no_position() {
        let mut snap = PaneSnapshot {
            id: 0,
            rows: 2,
            cols: 2,
            cells: Vec::new(),
            cursor_row: 1,
            cursor_col: 1,
            cursor_visible: true,
        };
        let inner = Rect::new(5, 5, 2, 2);
        assert_eq!(cursor_position(inner, &snap), Some((6, 6)));
        snap.cursor_visible = false;
        assert_eq!(cursor_position(inner, &snap), None);
    }

    #[test]
    fn cursor_past_the_right_margin_stays_on_the_last_column() {
        let mut parser = vt100::Parser::new(2, 4, 0);
        parser.process(b"abcd");
        let snap = crate::tui::pane::snapshot_screen(0, parser.screen());
        assert_eq!(snap.cursor_col, 4);
        let inner = Rect::new(1, 1, 4, 2);
        assert_eq!(cursor_position(inner, &snap), Some((4, 1)));
    }
}
