use anyhow::Result;
use crate::security::agent_spawn::AgentPtySpawn;
use ratatui::{buffer::Buffer, layout::Rect};

use ratatui::widgets::Widget;

use super::{
    layout::{Layout, ResizeDir},
    pane::Pane,
    protocol::{PaneLayout, WindowSnapshot},
    zoom::ZoomState,
};

/// A window holds one or more panes arranged in a BSP layout.
///
/// Responsibilities (Stories 1.2 & 1.3):
/// - Spawn and own `Pane` instances
/// - Split the focused pane horizontally or vertically
/// - Track which pane is focused
/// - Handle directional focus movement and pane resize
pub struct Window {
    pub id: usize,
    pub name: String,
    panes: Vec<Pane>,
    layout: Layout,
    focused_pane: usize,
    next_pane_id: usize,
    /// Total area allocated to this window (updated on resize/render)
    last_area: Rect,
    /// Current mouse hover position (for border highlight).
    hover_col: Option<u16>,
    hover_row: Option<u16>,
    zoom: ZoomState,
}

impl Window {
    /// Create a new window with a single pane.
    pub fn new(id: usize, rows: u16, cols: u16) -> Result<Self> {
        // Each pane's usable area is reduced by borders (2 chars each side)
        let inner_rows = rows.saturating_sub(2).max(1);
        let inner_cols = cols.saturating_sub(2).max(1);
        let pane = Pane::new(0, inner_rows, inner_cols)?;
        Ok(Self {
            id,
            name: (id + 1).to_string(),
            panes: vec![pane],
            layout: Layout::new(0),
            focused_pane: 0,
            next_pane_id: 1,
            last_area: Rect::new(0, 0, cols, rows),
            hover_col: None,
            hover_row: None,
            zoom: ZoomState::new(),
        })
    }

    pub fn is_zoomed(&self) -> bool {
        self.zoom.is_zoomed()
    }

    pub fn zoom_state(&self) -> ZoomState {
        self.zoom.clone()
    }

    pub fn scroll_active(&self) -> bool {
        self.pane_ref(self.focused_pane)
            .map(|p| p.scroll_active())
            .unwrap_or(false)
    }

    pub fn toggle_zoom(&mut self) {
        self.zoom.toggle(self.focused_pane);
    }

    pub fn enter_scroll_mode(&mut self) {
        if let Some(pane) = self.pane_mut(self.focused_pane) {
            pane.enter_scroll_mode();
        }
    }

    /// Returns true when input was consumed by scroll mode.
    pub fn handle_scroll_input(&self, bytes: &[u8]) -> bool {
        if let Some(pane) = self.pane_ref(self.focused_pane) {
            pane.handle_scroll_input(bytes)
        } else {
            false
        }
    }

    pub fn focused_pane_id(&self) -> usize {
        self.focused_pane
    }

    // ── Story 1.2: Pane splitting ─────────────────────────────────────────────

    /// Split the currently focused pane horizontally (left / right).
    pub fn split_horizontal(&mut self) -> Result<()> {
        let (rows, cols) = self.focused_inner_size();
        let new_id = self.next_pane_id;
        self.next_pane_id += 1;
        let pane = Pane::new(new_id, rows, cols / 2)?;
        self.panes.push(pane);
        self.layout = std::mem::replace(&mut self.layout, Layout::new(0))
            .split_h(self.focused_pane, new_id);
        self.focused_pane = new_id;
        Ok(())
    }

    /// Split horizontally and spawn a command in the new pane (structured agent spawn).
    pub fn split_horizontal_with_command(&mut self, spawn: AgentPtySpawn) -> Result<usize> {
        let (rows, cols) = self.focused_inner_size();
        let new_id = self.next_pane_id;
        self.next_pane_id += 1;
        let pane = Pane::spawn_with_command(
            new_id,
            rows,
            cols / 2,
            spawn.command,
            spawn.inject_fds,
        )?;
        self.panes.push(pane);
        self.layout = std::mem::replace(&mut self.layout, Layout::new(0))
            .split_h(self.focused_pane, new_id);
        self.focused_pane = new_id;
        Ok(new_id)
    }

    /// Split the currently focused pane vertically (top / bottom).
    pub fn split_vertical(&mut self) -> Result<()> {
        let (rows, cols) = self.focused_inner_size();
        let new_id = self.next_pane_id;
        self.next_pane_id += 1;
        let pane = Pane::new(new_id, rows / 2, cols)?;
        self.panes.push(pane);
        self.layout = std::mem::replace(&mut self.layout, Layout::new(0))
            .split_v(self.focused_pane, new_id);
        self.focused_pane = new_id;
        Ok(())
    }

    // ── Story 1.3: Navigation & resize ───────────────────────────────────────

    pub fn focus_left(&mut self) {
        if let Some(id) = self.layout.pane_to_left(self.focused_pane, self.last_area) {
            self.focused_pane = id;
        }
    }

    pub fn focus_right(&mut self) {
        if let Some(id) = self.layout.pane_to_right(self.focused_pane, self.last_area) {
            self.focused_pane = id;
        }
    }

    pub fn focus_up(&mut self) {
        if let Some(id) = self.layout.pane_above(self.focused_pane, self.last_area) {
            self.focused_pane = id;
        }
    }

    pub fn focus_down(&mut self) {
        if let Some(id) = self.layout.pane_below(self.focused_pane, self.last_area) {
            self.focused_pane = id;
        }
    }

    /// Find which pane contains the given screen coordinates.
    /// Returns the pane ID if found.
    pub fn pane_at(&self, col: u16, row: u16) -> Option<usize> {
        let rects = self.layout.compute_rects(self.last_area);
        for (id, rect) in &rects {
            if col >= rect.x && col < rect.x + rect.width
                && row >= rect.y && row < rect.y + rect.height
            {
                return Some(*id);
            }
        }
        None
    }

    /// Check if a screen position is on a vertical border between panes.
    /// Returns the pane IDs on left and right of the border, plus the border X.
    pub fn border_at(&self, col: u16, row: u16) -> Option<(usize, usize, u16)> {
        let rects = self.layout.compute_rects(self.last_area);
        // A vertical border is at the right edge of a pane
        for (id_left, rect_l) in &rects {
            let border_x = rect_l.x + rect_l.width;
            // Click within 1 column of the border
            if col >= border_x.saturating_sub(1) && col <= border_x
                && row >= rect_l.y && row < rect_l.y + rect_l.height
            {
                // Find the pane on the right side of this border
                for (id_right, rect_r) in &rects {
                    if rect_r.x == border_x
                        && row >= rect_r.y && row < rect_r.y + rect_r.height
                    {
                        return Some((*id_left, *id_right, border_x));
                    }
                }
            }
        }
        None
    }

    /// Set focus to a specific pane by ID.
    pub fn set_focus(&mut self, pane_id: usize) {
        if self.panes.iter().any(|p| p.id == pane_id) {
            self.focused_pane = pane_id;
        }
    }

    /// Cycle focus to the next pane (Ctrl-b o).
    pub fn focus_next_pane(&mut self) {
        if self.panes.is_empty() {
            return;
        }
        let ids: Vec<usize> = self.panes.iter().map(|p| p.id).collect();
        if let Some(pos) = ids.iter().position(|&id| id == self.focused_pane) {
            let next = (pos + 1) % ids.len();
            self.focused_pane = ids[next];
        }
    }

    /// Update the mouse hover position. Call on every mouse move event.
    pub fn set_hover(&mut self, col: u16, row: u16) {
        self.hover_col = Some(col);
        self.hover_row = Some(row);
    }

    /// Clear the mouse hover position.
    pub fn clear_hover(&mut self) {
        self.hover_col = None;
        self.hover_row = None;
    }

    /// Returns true if the mouse is currently hovering over a vertical pane border.
    fn is_hover_on_border(&self) -> bool {
        if let (Some(col), Some(row)) = (self.hover_col, self.hover_row) {
            self.border_at(col, row).is_some()
        } else {
            false
        }
    }

    pub fn resize_pane(&mut self, dir: ResizeDir) {
        self.layout.resize_pane(self.focused_pane, dir);
    }

    /// Set the horizontal split ratio for a border drag.
    /// `left_pane_id` is the pane on the left side of the dragged border.
    /// `mouse_x` is the current mouse column position.
    pub fn drag_border(&mut self, left_pane_id: usize, mouse_x: u16) {
        let area = self.last_area;
        if area.width == 0 { return; }
        self.layout.set_hsplit_ratio_at(left_pane_id, mouse_x, area);
    }

    // ── Input forwarding ──────────────────────────────────────────────────────

    /// Send raw bytes to the focused pane's PTY.
    pub fn send_input(&mut self, bytes: &[u8]) -> Result<()> {
        if let Some(pane) = self.pane_mut(self.focused_pane) {
            pane.send_input(bytes)?;
        }
        Ok(())
    }

    // ── Resize ────────────────────────────────────────────────────────────────

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        self.last_area = Rect::new(0, 0, cols, rows);
        let rects = self.layout.compute_rects(self.last_area);
        for (pane_id, rect) in rects {
            if let Some(pane) = self.pane_mut(pane_id) {
                let inner_rows = rect.height.saturating_sub(2).max(1);
                let inner_cols = rect.width.saturating_sub(2).max(1);
                let _ = pane.resize(inner_rows, inner_cols);
            }
        }
        Ok(())
    }

    // ── Output polling ────────────────────────────────────────────────────────

    /// Returns `true` if any pane has new PTY output since last call.
    pub fn poll_output(&self) -> bool {
        self.panes.iter().any(|p| p.take_new_output())
    }

    // ── Rendering ─────────────────────────────────────────────────────────────

    /// Render all panes into the frame buffer.
    pub fn render(&mut self, buf: &mut Buffer, area: Rect) {
        use ratatui::style::{Color, Style};

        self.last_area = area;

        if self.zoom.is_zoomed() {
            let pane_id = self.zoom.zoomed_pane_id().unwrap_or(self.focused_pane);
            if let Some(pane) = self.pane_ref(pane_id) {
                pane.view(true).render(area, buf);
            }
        } else {
            let rects = self.layout.compute_rects(area);
            for (pane_id, rect) in &rects {
                if !self.zoom.is_pane_visible(*pane_id) {
                    continue;
                }
                let focused = *pane_id == self.focused_pane;
                if let Some(pane) = self.pane_ref(*pane_id) {
                    pane.view(focused).render(*rect, buf);
                }
            }
        }

        // Highlight vertical borders when mouse hovers on them.
        // Draw a bright column where two panes share an edge.
        if self.is_hover_on_border() {
            if let (Some(hcol), Some(_hrow)) = (self.hover_col, self.hover_row) {
                if let Some((_left_id, _right_id, border_x)) = self.border_at(hcol, _hrow) {
                    // Highlight the entire vertical border column
                    let border_style = Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan);
                    for y in area.y..area.y + area.height {
                        if border_x > 0 && border_x < area.x + area.width {
                            let cell = &mut buf[(border_x - 1, y)];
                            cell.set_style(border_style);
                            cell.set_symbol("┃");
                        }
                    }
                }
            }
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Capture a serializable snapshot for the daemon protocol.
    pub fn snapshot(&self) -> WindowSnapshot {
        let (pane_layouts, pane_cells) = if self.zoom.is_zoomed() {
            let pane_id = self.zoom.zoomed_pane_id().unwrap_or(self.focused_pane);
            let area = self.last_area;
            let layouts = vec![PaneLayout {
                id: pane_id,
                x: area.x,
                y: area.y,
                width: area.width,
                height: area.height,
            }];
            let cells = self
                .panes
                .iter()
                .filter(|p| p.id == pane_id)
                .map(|p| p.snapshot())
                .collect();
            (layouts, cells)
        } else {
            let rects = self.layout.compute_rects(self.last_area);
            let layouts: Vec<PaneLayout> = rects
                .iter()
                .filter(|(id, _)| self.zoom.is_pane_visible(*id))
                .map(|(id, r)| PaneLayout {
                    id: *id,
                    x: r.x,
                    y: r.y,
                    width: r.width,
                    height: r.height,
                })
                .collect();
            let cells: Vec<_> = self
                .panes
                .iter()
                .filter(|p| self.zoom.is_pane_visible(p.id))
                .map(|p| p.snapshot())
                .collect();
            (layouts, cells)
        };
        WindowSnapshot {
            id: self.id,
            name: self.name.clone(),
            pane_layouts,
            pane_cells,
            focused_pane: self.focused_pane,
            zoomed: self.zoom.is_zoomed(),
            scroll_active: self.scroll_active(),
        }
    }

    fn pane_ref(&self, id: usize) -> Option<&Pane> {
        self.panes.iter().find(|p| p.id == id)
    }

    /// Get a mutable reference to a pane by ID.
    pub fn pane_mut(&mut self, id: usize) -> Option<&mut Pane> {
        self.panes.iter_mut().find(|p| p.id == id)
    }

    /// Send input bytes to a specific pane (not necessarily the focused one).
    pub fn send_input_to_pane(&mut self, pane_id: usize, bytes: &[u8]) -> Result<()> {
        if let Some(pane) = self.pane_mut(pane_id) {
            pane.send_input(bytes)?;
        }
        Ok(())
    }

    fn focused_inner_size(&self) -> (u16, u16) {
        let rects = self.layout.compute_rects(self.last_area);
        if let Some((_, rect)) = rects.iter().find(|(id, _)| *id == self.focused_pane) {
            let rows = rect.height.saturating_sub(2).max(1);
            let cols = rect.width.saturating_sub(2).max(1);
            (rows, cols)
        } else {
            (24, 80)
        }
    }
}
