use bmux::tui::zoom::ZoomState;

#[test]
fn integration_zoom_toggle_hides_other_panes() {
    let mut zoom = ZoomState::new();
    let panes = [1usize, 2, 3, 4];

    // Initially all panes visible
    for &p in &panes {
        assert!(zoom.is_pane_visible(p));
    }

    // Zoom pane 3
    zoom.toggle(3);
    for &p in &panes {
        if p == 3 {
            assert!(zoom.is_pane_visible(p), "Pane {p} should be visible");
        } else {
            assert!(!zoom.is_pane_visible(p), "Pane {p} should be hidden");
        }
    }

    // Un-zoom — all panes visible again
    zoom.toggle(3);
    for &p in &panes {
        assert!(zoom.is_pane_visible(p), "Pane {p} should be visible after un-zoom");
    }
}

#[test]
fn integration_zoom_content_preserved_on_toggle() {
    // Zoom doesn't kill panes — only hides them visually.
    // The pane IDs remain valid after un-zoom.
    let mut zoom = ZoomState::new();
    zoom.toggle(1);
    assert_eq!(zoom.zoomed_pane_id(), Some(1));
    zoom.toggle(1);
    // After un-zoom: state is clean
    assert!(!zoom.is_zoomed());
    assert_eq!(zoom.zoomed_pane_id(), None);
    // Pane 1 still "exists" (not killed)
    assert!(zoom.is_pane_visible(1));
}
