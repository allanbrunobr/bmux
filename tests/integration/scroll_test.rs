use bmux::tui::scroll::{ScrollState, DEFAULT_HISTORY_LIMIT};

#[test]
fn integration_scroll_full_workflow() {
    let mut s = ScrollState::new();

    // Fill scrollback with content
    for i in 0..100 {
        s.push_line(format!("output line {i}"));
    }
    assert_eq!(s.buffer_len(), 100);
    assert!(!s.is_active());

    // Enter scroll mode (Ctrl-b [)
    s.enter();
    assert!(s.is_active());
    assert_eq!(s.indicator(), Some("[SCROLL]"));
    assert_eq!(s.offset(), 0);

    // Scroll up with k
    s.scroll_up(10);
    assert_eq!(s.offset(), 10);

    // Scroll down with j
    s.scroll_down(5);
    assert_eq!(s.offset(), 5);

    // Exit with q or Escape
    s.exit();
    assert!(!s.is_active());
    assert_eq!(s.offset(), 0);
}

#[test]
fn integration_history_limit_enforced() {
    let mut s = ScrollState::new();
    assert!(
        DEFAULT_HISTORY_LIMIT >= 10_000,
        "AC: buffer must support at least 10,000 lines"
    );

    // Push exactly 10,000 lines
    for i in 0..10_000 {
        s.push_line(format!("line {i}"));
    }
    assert_eq!(s.buffer_len(), 10_000);

    // Push one more — oldest should drop, count stays at 10,000
    s.push_line("extra");
    assert_eq!(s.buffer_len(), 10_000);
}

#[test]
fn integration_scroll_clamps_correctly() {
    let mut s = ScrollState::new();
    for i in 0..20 {
        s.push_line(format!("L{i}"));
    }
    s.enter();

    // Scroll way past the top
    s.scroll_up(9999);
    assert_eq!(s.offset(), 19); // max = buffer_len - 1

    // Scroll way past the bottom
    s.scroll_down(9999);
    assert_eq!(s.offset(), 0);
}
