use anyhow::{Context, Result};
use portable_pty::{CommandBuilder, MasterPty, PtySize};
#[cfg(unix)]
use std::os::unix::io::RawFd;
#[cfg(not(unix))]
type RawFd = i32;

use super::pty_spawn;
use super::{
    chrome::{cursor_position, pane_block, render_pane_cells},
    protocol::{CellColor, CellData, PaneSnapshot},
    scroll::ScrollState,
};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use std::{
    io::Write,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

/// A single terminal pane backed by a pseudoterminal.
///
/// Each pane:
/// - owns a PTY master connected to a child shell process
/// - runs a background thread that feeds PTY output into a `vt100::Parser`
/// - can be rendered as a ratatui `Widget` (with or without active border)
pub struct Pane {
    pub id: usize,
    parser: Arc<Mutex<vt100::Parser>>,
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send>,
    /// Set to `true` by the reader thread whenever new PTY output arrives.
    pub has_output: Arc<AtomicBool>,
    scroll: Arc<Mutex<ScrollState>>,
}

/// Shell for new panes, set from config (`[tui] default_shell`).
static DEFAULT_SHELL: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);

/// Set the shell used by new panes. `None` restores `$SHELL`.
pub fn set_default_shell(shell: Option<String>) {
    let shell = shell.filter(|s| !s.trim().is_empty());
    if let Ok(mut guard) = DEFAULT_SHELL.write() {
        *guard = shell;
    }
}

/// Shell for new panes: configured shell, else `$SHELL`, else `/bin/sh`.
pub fn default_shell() -> String {
    DEFAULT_SHELL
        .read()
        .ok()
        .and_then(|g| g.clone())
        .or_else(|| std::env::var("SHELL").ok())
        .unwrap_or_else(|| "/bin/sh".to_string())
}

impl Pane {
    /// Spawn a new pane running the configured shell (see [`default_shell`]).
    pub fn new(id: usize, rows: u16, cols: u16) -> Result<Self> {
        let shell = default_shell();
        Self::spawn_with_command(id, rows, cols, CommandBuilder::new(&shell), Vec::new())
    }

    /// Spawn a pane with a pre-built command (structured agent spawn — no shell injection).
    pub fn spawn_with_command(
        id: usize,
        rows: u16,
        cols: u16,
        cmd: CommandBuilder,
        inject_fds: Vec<RawFd>,
    ) -> Result<Self> {
        let spawned = pty_spawn::spawn_with_preserved_fds(rows, cols, cmd, inject_fds)
            .context("Failed to spawn command in PTY")?;

        for fd in spawned.parent_fds_to_close {
            unsafe {
                libc::close(fd);
            }
        }

        let writer = spawned
            .master
            .take_writer()
            .context("Failed to acquire PTY writer")?;
        let mut reader = spawned
            .master
            .try_clone_reader()
            .context("Failed to acquire PTY reader")?;

        let parser = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 10_000)));
        let has_output = Arc::new(AtomicBool::new(false));
        let scroll = Arc::new(Mutex::new(ScrollState::new()));

        {
            let parser_clone = Arc::clone(&parser);
            let flag_clone = Arc::clone(&has_output);
            let scroll_clone = Arc::clone(&scroll);
            std::thread::Builder::new()
                .name(format!("bmux-pty-reader-{id}"))
                .spawn(move || {
                    let mut buf = [0u8; 4096];
                    let mut line_buf: Vec<u8> = Vec::new();
                    loop {
                        match reader.read(&mut buf) {
                            Ok(0) | Err(_) => break,
                            Ok(n) => {
                                for &byte in &buf[..n] {
                                    if byte == b'\n' {
                                        if !line_buf.is_empty() {
                                            let line =
                                                String::from_utf8_lossy(&line_buf).to_string();
                                            scroll_clone.lock().unwrap().push_line(line);
                                            line_buf.clear();
                                        }
                                    } else if byte != b'\r' {
                                        line_buf.push(byte);
                                    }
                                }
                                parser_clone.lock().unwrap().process(&buf[..n]);
                                flag_clone.store(true, Ordering::Relaxed);
                            }
                        }
                    }
                })
                .expect("Failed to spawn PTY reader thread");
        }

        Ok(Self {
            id,
            parser,
            writer,
            master: spawned.master,
            child: spawned.child,
            has_output,
            scroll,
        })
    }

    pub fn scroll_active(&self) -> bool {
        self.scroll.lock().unwrap().is_active()
    }

    pub fn enter_scroll_mode(&self) {
        self.scroll.lock().unwrap().enter();
    }

    pub fn exit_scroll_mode(&self) {
        self.scroll.lock().unwrap().exit();
    }

    /// Handle scroll-mode keys. Returns true if the input was consumed.
    pub fn handle_scroll_input(&self, bytes: &[u8]) -> bool {
        let mut scroll = self.scroll.lock().unwrap();
        if !scroll.is_active() {
            return false;
        }
        match bytes {
            b"k" | b"\x1b[A" | b"\x1bOA" => scroll.scroll_up(1),
            b"j" | b"\x1b[B" | b"\x1bOB" => scroll.scroll_down(1),
            b"q" | b"\x1b" => scroll.exit(),
            _ => return false,
        }
        true
    }

    /// Child process ID when available from the PTY backend.
    pub fn process_id(&self) -> Option<u32> {
        self.child.process_id()
    }

    /// Send raw input bytes to the PTY (keyboard input forwarding).
    pub fn send_input(&mut self, bytes: &[u8]) -> Result<()> {
        self.writer
            .write_all(bytes)
            .context("Failed to write to PTY")?;
        self.writer.flush()?;
        Ok(())
    }

    /// Returns `true` and clears the flag if new PTY output has arrived since last call.
    pub fn take_new_output(&self) -> bool {
        self.has_output.swap(false, Ordering::Relaxed)
    }

    /// Resize the PTY and update the terminal emulator dimensions.
    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to resize PTY")?;
        self.parser.lock().unwrap().set_size(rows, cols);
        Ok(())
    }

    /// Kill the child process.
    pub fn kill(&mut self) -> Result<()> {
        self.child.kill().context("Failed to kill child process")
    }

    /// Create a renderable view for this pane.
    pub fn view(&self, focused: bool) -> PaneView<'_> {
        PaneView {
            pane: self,
            focused,
        }
    }

    /// Render screen content into an already-computed inner rect (no
    /// borders). Returns the absolute terminal cursor position when the
    /// application shows its cursor (never in scroll mode).
    pub fn render_content(&self, buf: &mut Buffer, inner: Rect) -> Option<(u16, u16)> {
        let snap = self.snapshot();
        let inner = inner.intersection(buf.area);
        render_pane_cells(buf, inner, &snap);
        cursor_position(inner, &snap)
    }

    /// Draw the rounded border and the content. Returns the cursor position
    /// (see [`Pane::render_content`]).
    pub fn render_framed(&self, area: Rect, buf: &mut Buffer, focused: bool) -> Option<(u16, u16)> {
        let block = pane_block(focused);
        let inner = block.inner(area);
        block.render(area, buf);
        self.render_content(buf, inner)
    }
}

/// A renderable view of a pane, implementing ratatui's `Widget` trait.
pub struct PaneView<'a> {
    pane: &'a Pane,
    focused: bool,
}

impl Widget for PaneView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.pane.render_framed(area, buf, self.focused);
    }
}

impl Pane {
    /// Capture a serializable snapshot of the current screen state.
    pub fn snapshot(&self) -> PaneSnapshot {
        if self.scroll.lock().unwrap().is_active() {
            return self.snapshot_scroll();
        }
        let parser = self.parser.lock().unwrap();
        snapshot_screen(self.id, parser.screen())
    }

    /// Snapshot of the scroll-back view, sized like the live screen.
    fn snapshot_scroll(&self) -> PaneSnapshot {
        let (rows, cols) = self.parser.lock().unwrap().screen().size();
        let scroll = self.scroll.lock().unwrap();
        let lines = scroll.visible_lines(rows as usize);
        let mut cells = Vec::with_capacity(rows as usize);
        for r in 0..rows as usize {
            let line = lines.get(r).copied().unwrap_or("");
            let mut chars = line.chars();
            let row_cells = (0..cols)
                .map(|_| match chars.next() {
                    Some(ch) if !ch.is_control() => CellData {
                        ch: ch.to_string(),
                        ..CellData::default()
                    },
                    _ => CellData::default(),
                })
                .collect();
            cells.push(row_cells);
        }
        PaneSnapshot {
            id: self.id,
            rows,
            cols,
            cells,
            cursor_row: 0,
            cursor_col: 0,
            cursor_visible: false,
        }
    }
} // end impl Pane (snapshot)

/// Build a snapshot from a vt100 screen (colours and attributes kept 1:1).
pub fn snapshot_screen(id: usize, screen: &vt100::Screen) -> PaneSnapshot {
    let (rows, cols) = screen.size();
    let (cursor_row, cursor_col) = screen.cursor_position();
    let mut cells = Vec::with_capacity(rows as usize);
    for r in 0..rows {
        let mut row_cells = Vec::with_capacity(cols as usize);
        for c in 0..cols {
            row_cells.push(screen.cell(r, c).map(cell_data).unwrap_or_default());
        }
        cells.push(row_cells);
    }
    PaneSnapshot {
        id,
        rows,
        cols,
        cells,
        cursor_row,
        cursor_col,
        cursor_visible: !screen.hide_cursor(),
    }
}

fn cell_data(cell: &vt100::Cell) -> CellData {
    let contents = cell.contents();
    CellData {
        ch: if contents.is_empty() {
            " ".to_string()
        } else {
            contents
        },
        fg: cell_color(cell.fgcolor()),
        bg: cell_color(cell.bgcolor()),
        bold: cell.bold(),
        italic: cell.italic(),
        underline: cell.underline(),
        inverse: cell.inverse(),
        wide_cont: cell.is_wide_continuation(),
    }
}

fn cell_color(c: vt100::Color) -> CellColor {
    match c {
        vt100::Color::Default => CellColor::Default,
        vt100::Color::Idx(i) => CellColor::Idx(i),
        vt100::Color::Rgb(r, g, b) => CellColor::Rgb(r, g, b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap_of(input: &[u8]) -> PaneSnapshot {
        let mut parser = vt100::Parser::new(4, 10, 0);
        parser.process(input);
        snapshot_screen(7, parser.screen())
    }

    #[test]
    fn indexed_256_colour_is_kept() {
        let s = snap_of(b"\x1b[38;5;196mX");
        assert_eq!(s.cells[0][0].ch, "X");
        assert_eq!(s.cells[0][0].fg, CellColor::Idx(196));
    }

    #[test]
    fn rgb_colour_is_kept() {
        let s = snap_of(b"\x1b[38;2;1;2;3mX");
        assert_eq!(s.cells[0][0].fg, CellColor::Rgb(1, 2, 3));
    }

    #[test]
    fn explicit_black_is_not_default() {
        let s = snap_of(b"\x1b[30mX\x1b[0mY");
        assert_eq!(s.cells[0][0].fg, CellColor::Idx(0));
        assert_eq!(s.cells[0][1].fg, CellColor::Default);
    }

    #[test]
    fn inverse_italic_underline_are_kept() {
        let s = snap_of(b"\x1b[7mA\x1b[0;3mB\x1b[0;4mC");
        assert!(s.cells[0][0].inverse);
        assert!(s.cells[0][1].italic);
        assert!(s.cells[0][2].underline);
        assert!(!s.cells[0][2].inverse);
    }

    #[test]
    fn cursor_visibility_follows_the_application() {
        let s = snap_of(b"ab");
        assert!(s.cursor_visible);
        assert_eq!((s.cursor_row, s.cursor_col), (0, 2));
        let s = snap_of(b"\x1b[?25l");
        assert!(!s.cursor_visible);
    }

    #[test]
    fn wide_char_marks_continuation_cell() {
        let s = snap_of("界x".as_bytes());
        assert_eq!(s.cells[0][0].ch, "界");
        assert!(!s.cells[0][0].wide_cont);
        assert!(s.cells[0][1].wide_cont);
        assert_eq!(s.cells[0][2].ch, "x");
    }

    #[test]
    fn combining_grapheme_is_not_truncated() {
        let s = snap_of("e\u{301}".as_bytes());
        assert_eq!(s.cells[0][0].ch, "e\u{301}");
    }

    #[test]
    fn empty_cells_are_default() {
        let s = snap_of(b"");
        assert_eq!(s.rows, 4);
        assert_eq!(s.cols, 10);
        assert_eq!(s.cells[3][9], CellData::default());
    }
}
