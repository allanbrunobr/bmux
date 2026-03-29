use ratatui::layout::Rect;

/// Increment (as a fraction of total size) used for H/J/K/L resizes.
const RESIZE_STEP: f32 = 0.05;

/// Direction for pane resize operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeDir {
    Left,
    Right,
    Up,
    Down,
}

/// Binary-space-partition layout tree.
///
/// Each leaf node holds a pane ID.
/// `HSplit` divides space left/right; `VSplit` divides top/bottom.
/// `ratio` is the fraction of space given to the first child (left or top).
#[derive(Debug, Clone)]
pub enum Layout {
    Leaf(usize),
    HSplit {
        left: Box<Layout>,
        right: Box<Layout>,
        ratio: f32,
    },
    VSplit {
        top: Box<Layout>,
        bottom: Box<Layout>,
        ratio: f32,
    },
}

impl Layout {
    /// Single-pane layout.
    pub fn new(pane_id: usize) -> Self {
        Layout::Leaf(pane_id)
    }

    /// Split the leaf with `target_id` horizontally (creates left/right siblings).
    /// The new pane (`new_id`) becomes the right child.
    pub fn split_h(self, target_id: usize, new_id: usize) -> Self {
        match self {
            Layout::Leaf(id) if id == target_id => Layout::HSplit {
                left: Box::new(Layout::Leaf(id)),
                right: Box::new(Layout::Leaf(new_id)),
                ratio: 0.5,
            },
            Layout::Leaf(_) => self,
            Layout::HSplit { left, right, ratio } => Layout::HSplit {
                left: Box::new(left.split_h(target_id, new_id)),
                right: Box::new(right.split_h(target_id, new_id)),
                ratio,
            },
            Layout::VSplit { top, bottom, ratio } => Layout::VSplit {
                top: Box::new(top.split_h(target_id, new_id)),
                bottom: Box::new(bottom.split_h(target_id, new_id)),
                ratio,
            },
        }
    }

    /// Split the leaf with `target_id` vertically (creates top/bottom siblings).
    /// The new pane (`new_id`) becomes the bottom child.
    pub fn split_v(self, target_id: usize, new_id: usize) -> Self {
        match self {
            Layout::Leaf(id) if id == target_id => Layout::VSplit {
                top: Box::new(Layout::Leaf(id)),
                bottom: Box::new(Layout::Leaf(new_id)),
                ratio: 0.5,
            },
            Layout::Leaf(_) => self,
            Layout::HSplit { left, right, ratio } => Layout::HSplit {
                left: Box::new(left.split_v(target_id, new_id)),
                right: Box::new(right.split_v(target_id, new_id)),
                ratio,
            },
            Layout::VSplit { top, bottom, ratio } => Layout::VSplit {
                top: Box::new(top.split_v(target_id, new_id)),
                bottom: Box::new(bottom.split_v(target_id, new_id)),
                ratio,
            },
        }
    }

    /// Return all pane IDs in left-to-right, top-to-bottom order.
    pub fn pane_ids(&self) -> Vec<usize> {
        match self {
            Layout::Leaf(id) => vec![*id],
            Layout::HSplit { left, right, .. } => {
                let mut v = left.pane_ids();
                v.extend(right.pane_ids());
                v
            }
            Layout::VSplit { top, bottom, .. } => {
                let mut v = top.pane_ids();
                v.extend(bottom.pane_ids());
                v
            }
        }
    }

    /// Compute the screen `Rect` for every pane given the total available area.
    pub fn compute_rects(&self, area: Rect) -> Vec<(usize, Rect)> {
        match self {
            Layout::Leaf(id) => vec![(*id, area)],
            Layout::HSplit { left, right, ratio } => {
                let split = ((area.width as f32 * ratio) as u16).max(1).min(area.width - 1);
                let left_rect = Rect::new(area.x, area.y, split, area.height);
                let right_rect =
                    Rect::new(area.x + split, area.y, area.width - split, area.height);
                let mut v = left.compute_rects(left_rect);
                v.extend(right.compute_rects(right_rect));
                v
            }
            Layout::VSplit { top, bottom, ratio } => {
                let split = ((area.height as f32 * ratio) as u16).max(1).min(area.height - 1);
                let top_rect = Rect::new(area.x, area.y, area.width, split);
                let bot_rect =
                    Rect::new(area.x, area.y + split, area.width, area.height - split);
                let mut v = top.compute_rects(top_rect);
                v.extend(bottom.compute_rects(bot_rect));
                v
            }
        }
    }

    /// Return true if this subtree contains the given pane ID.
    pub fn contains(&self, pane_id: usize) -> bool {
        match self {
            Layout::Leaf(id) => *id == pane_id,
            Layout::HSplit { left, right, .. } => {
                left.contains(pane_id) || right.contains(pane_id)
            }
            Layout::VSplit { top, bottom, .. } => {
                top.contains(pane_id) || bottom.contains(pane_id)
            }
        }
    }

    // ── Directional navigation ────────────────────────────────────────────────

    fn rect_of(&self, pane_id: usize, area: Rect) -> Option<Rect> {
        self.compute_rects(area)
            .into_iter()
            .find(|(id, _)| *id == pane_id)
            .map(|(_, r)| r)
    }

    /// The pane immediately to the left of `pane_id`, or `None` at the edge.
    pub fn pane_to_left(&self, pane_id: usize, area: Rect) -> Option<usize> {
        let rects = self.compute_rects(area);
        let cur = rects.iter().find(|(id, _)| *id == pane_id)?.1;
        let cy = cur.y + cur.height / 2;
        rects
            .iter()
            .filter(|(id, r)| {
                *id != pane_id
                    && r.x + r.width <= cur.x
                    && r.y <= cy
                    && cy < r.y + r.height
            })
            .max_by_key(|(_, r)| r.x + r.width)
            .map(|(id, _)| *id)
    }

    /// The pane immediately to the right of `pane_id`, or `None` at the edge.
    pub fn pane_to_right(&self, pane_id: usize, area: Rect) -> Option<usize> {
        let rects = self.compute_rects(area);
        let cur = rects.iter().find(|(id, _)| *id == pane_id)?.1;
        let cy = cur.y + cur.height / 2;
        rects
            .iter()
            .filter(|(id, r)| {
                *id != pane_id
                    && r.x >= cur.x + cur.width
                    && r.y <= cy
                    && cy < r.y + r.height
            })
            .min_by_key(|(_, r)| r.x)
            .map(|(id, _)| *id)
    }

    /// The pane immediately above `pane_id`, or `None` at the edge.
    pub fn pane_above(&self, pane_id: usize, area: Rect) -> Option<usize> {
        let rects = self.compute_rects(area);
        let cur = rects.iter().find(|(id, _)| *id == pane_id)?.1;
        let cx = cur.x + cur.width / 2;
        rects
            .iter()
            .filter(|(id, r)| {
                *id != pane_id
                    && r.y + r.height <= cur.y
                    && r.x <= cx
                    && cx < r.x + r.width
            })
            .max_by_key(|(_, r)| r.y + r.height)
            .map(|(id, _)| *id)
    }

    /// The pane immediately below `pane_id`, or `None` at the edge.
    pub fn pane_below(&self, pane_id: usize, area: Rect) -> Option<usize> {
        let rects = self.compute_rects(area);
        let cur = rects.iter().find(|(id, _)| *id == pane_id)?.1;
        let cx = cur.x + cur.width / 2;
        rects
            .iter()
            .filter(|(id, r)| {
                *id != pane_id
                    && r.y >= cur.y + cur.height
                    && r.x <= cx
                    && cx < r.x + r.width
            })
            .min_by_key(|(_, r)| r.y)
            .map(|(id, _)| *id)
    }

    // ── Resize ────────────────────────────────────────────────────────────────

    /// Adjust the split ratio that governs `pane_id` in the given direction.
    /// If no applicable split exists the layout is returned unchanged.
    pub fn resize_pane(&mut self, pane_id: usize, dir: ResizeDir) {
        self.resize_node(pane_id, dir);
    }

    /// Set the split ratio of an HSplit that contains the given left pane.
    /// `ratio` is clamped to [0.1, 0.9].
    pub fn set_hsplit_ratio_for(&mut self, left_pane_id: usize, new_ratio: f32) {
        match self {
            Layout::Leaf(_) => {}
            Layout::HSplit { left, right: _, ratio } => {
                if left.contains(left_pane_id) {
                    *ratio = new_ratio.clamp(0.1, 0.9);
                    return;
                }
                left.set_hsplit_ratio_for(left_pane_id, new_ratio);
            }
            Layout::VSplit { top, bottom, .. } => {
                top.set_hsplit_ratio_for(left_pane_id, new_ratio);
                bottom.set_hsplit_ratio_for(left_pane_id, new_ratio);
            }
        }
    }

    fn resize_node(&mut self, pane_id: usize, dir: ResizeDir) -> bool {
        match self {
            Layout::Leaf(id) => *id == pane_id,
            Layout::HSplit { left, right, ratio } => {
                // Determine which child contains the target pane
                let in_left = left.contains(pane_id);
                let in_right = right.contains(pane_id);

                if in_left {
                    match dir {
                        ResizeDir::Right => {
                            *ratio = (*ratio + RESIZE_STEP).min(0.9);
                            return true;
                        }
                        ResizeDir::Left => {
                            *ratio = (*ratio - RESIZE_STEP).max(0.1);
                            return true;
                        }
                        _ => {}
                    }
                    // Propagate into subtree for Up/Down
                    left.resize_node(pane_id, dir)
                } else if in_right {
                    match dir {
                        ResizeDir::Left => {
                            // shrink right = grow left
                            *ratio = (*ratio + RESIZE_STEP).min(0.9);
                            return true;
                        }
                        ResizeDir::Right => {
                            *ratio = (*ratio - RESIZE_STEP).max(0.1);
                            return true;
                        }
                        _ => {}
                    }
                    right.resize_node(pane_id, dir)
                } else {
                    false
                }
            }
            Layout::VSplit { top, bottom, ratio } => {
                let in_top = top.contains(pane_id);
                let in_bottom = bottom.contains(pane_id);

                if in_top {
                    match dir {
                        ResizeDir::Down => {
                            *ratio = (*ratio + RESIZE_STEP).min(0.9);
                            return true;
                        }
                        ResizeDir::Up => {
                            *ratio = (*ratio - RESIZE_STEP).max(0.1);
                            return true;
                        }
                        _ => {}
                    }
                    top.resize_node(pane_id, dir)
                } else if in_bottom {
                    match dir {
                        ResizeDir::Up => {
                            *ratio = (*ratio + RESIZE_STEP).min(0.9);
                            return true;
                        }
                        ResizeDir::Down => {
                            *ratio = (*ratio - RESIZE_STEP).max(0.1);
                            return true;
                        }
                        _ => {}
                    }
                    bottom.resize_node(pane_id, dir)
                } else {
                    false
                }
            }
        }
    }
}
