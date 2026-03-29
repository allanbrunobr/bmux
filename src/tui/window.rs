use anyhow::Result;
use ratatui::{buffer::Buffer, layout::Rect};

use ratatui::widgets::Widget;

use super::{
    layout::{Layout, ResizeDir},
    pane::Pane,
    protocol::{PaneLayout, WindowSnapshot},
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
        })
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

    pub fn resize_pane(&mut self, dir: ResizeDir) {
        self.layout.resize_pane(self.focused_pane, dir);
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
        self.last_area = area;
        let rects = self.layout.compute_rects(area);
        for (pane_id, rect) in rects {
            let focused = pane_id == self.focused_pane;
            if let Some(pane) = self.pane_ref(pane_id) {
                pane.view(focused).render(rect, buf);
            }
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Capture a serializable snapshot for the daemon protocol.
    pub fn snapshot(&self) -> WindowSnapshot {
        let rects = self.layout.compute_rects(self.last_area);
        let pane_layouts: Vec<PaneLayout> = rects
            .iter()
            .map(|(id, r)| PaneLayout { id: *id, x: r.x, y: r.y, width: r.width, height: r.height })
            .collect();
        let pane_cells = self.panes.iter().map(|p| p.snapshot()).collect();
        WindowSnapshot {
            id: self.id,
            name: self.name.clone(),
            pane_layouts,
            pane_cells,
            focused_pane: self.focused_pane,
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
