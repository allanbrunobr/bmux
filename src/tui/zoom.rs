/// Tracks the zoom state of a window's pane layout.
///
/// When zoomed, the active pane fills the entire window area.
/// Other panes are hidden but not killed — their state is fully preserved.
/// Pressing Ctrl-b z again restores the original multi-pane layout.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ZoomState {
    /// Whether zoom is currently active
    is_zoomed: bool,
    /// The pane ID that is currently zoomed (None when not zoomed)
    zoomed_pane_id: Option<usize>,
}

impl ZoomState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if zoom is currently active.
    pub fn is_zoomed(&self) -> bool {
        self.is_zoomed
    }

    /// Returns the ID of the currently zoomed pane, if any.
    pub fn zoomed_pane_id(&self) -> Option<usize> {
        self.zoomed_pane_id
    }

    /// Toggle zoom for the given pane ID.
    ///
    /// - If not zoomed: zooms the given pane (other panes are hidden, not killed).
    /// - If zoomed on the same pane: restores the original layout.
    /// - If zoomed on a different pane: switches zoom to the new pane.
    pub fn toggle(&mut self, active_pane_id: usize) {
        if self.is_zoomed && self.zoomed_pane_id == Some(active_pane_id) {
            // Un-zoom: restore multi-pane layout
            self.is_zoomed = false;
            self.zoomed_pane_id = None;
        } else {
            // Zoom the active pane
            self.is_zoomed = true;
            self.zoomed_pane_id = Some(active_pane_id);
        }
    }

    /// Forcefully exit zoom mode (e.g., when the zoomed pane is closed).
    pub fn exit_zoom(&mut self) {
        self.is_zoomed = false;
        self.zoomed_pane_id = None;
    }

    /// Returns `true` if `pane_id` should be rendered (visible).
    ///
    /// When zoomed, only the zoomed pane is rendered; all others are hidden.
    /// When not zoomed, all panes are visible.
    pub fn is_pane_visible(&self, pane_id: usize) -> bool {
        if !self.is_zoomed {
            return true;
        }
        self.zoomed_pane_id == Some(pane_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_is_not_zoomed() {
        let state = ZoomState::new();
        assert!(!state.is_zoomed());
        assert_eq!(state.zoomed_pane_id(), None);
    }

    #[test]
    fn toggle_zooms_active_pane() {
        let mut state = ZoomState::new();
        state.toggle(1);
        assert!(state.is_zoomed());
        assert_eq!(state.zoomed_pane_id(), Some(1));
    }

    #[test]
    fn toggle_twice_restores_layout() {
        let mut state = ZoomState::new();
        state.toggle(1);
        state.toggle(1); // un-zoom
        assert!(!state.is_zoomed());
        assert_eq!(state.zoomed_pane_id(), None);
    }

    #[test]
    fn toggle_different_pane_switches_zoom() {
        let mut state = ZoomState::new();
        state.toggle(1);
        state.toggle(2); // switch to pane 2
        assert!(state.is_zoomed());
        assert_eq!(state.zoomed_pane_id(), Some(2));
    }

    #[test]
    fn exit_zoom_clears_state() {
        let mut state = ZoomState::new();
        state.toggle(3);
        state.exit_zoom();
        assert!(!state.is_zoomed());
        assert_eq!(state.zoomed_pane_id(), None);
    }

    #[test]
    fn pane_visibility_when_not_zoomed() {
        let state = ZoomState::new();
        // All panes are visible when not zoomed
        assert!(state.is_pane_visible(1));
        assert!(state.is_pane_visible(2));
        assert!(state.is_pane_visible(99));
    }

    #[test]
    fn pane_visibility_when_zoomed() {
        let mut state = ZoomState::new();
        state.toggle(2);
        // Only the zoomed pane is visible
        assert!(!state.is_pane_visible(1));
        assert!(state.is_pane_visible(2));
        assert!(!state.is_pane_visible(3));
    }

    #[test]
    fn pane_visibility_restored_after_unzoom() {
        let mut state = ZoomState::new();
        state.toggle(1);
        state.toggle(1); // un-zoom
        // All panes visible again
        assert!(state.is_pane_visible(1));
        assert!(state.is_pane_visible(2));
    }

    #[test]
    fn multiple_toggles_consistent() {
        let mut state = ZoomState::new();
        for i in 0..10 {
            state.toggle(5);
            if i % 2 == 0 {
                assert!(state.is_zoomed());
            } else {
                assert!(!state.is_zoomed());
            }
        }
    }

    #[test]
    fn ctrl_b_z_pattern_zoom_hide_other_panes() {
        // Simulates: user has panes 1,2,3 and presses Ctrl-b z on pane 2
        let mut state = ZoomState::new();
        let panes = vec![1usize, 2, 3];
        let active = 2usize;

        state.toggle(active);

        // Pane 2 visible (zoomed), others hidden
        for &p in &panes {
            if p == active {
                assert!(state.is_pane_visible(p), "pane {p} should be visible");
            } else {
                assert!(!state.is_pane_visible(p), "pane {p} should be hidden");
            }
        }

        // Press Ctrl-b z again — restore all
        state.toggle(active);
        for &p in &panes {
            assert!(state.is_pane_visible(p), "pane {p} should be visible after un-zoom");
        }
    }
}
