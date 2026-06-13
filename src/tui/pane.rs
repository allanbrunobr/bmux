use anyhow::{Context, Result};
use portable_pty::{CommandBuilder, MasterPty, PtySize};
#[cfg(unix)]
use std::os::unix::io::RawFd;
#[cfg(not(unix))]
type RawFd = i32;

use super::pty_spawn;
use super::{protocol::{CellData, PaneSnapshot}, scroll::ScrollState};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Widget},
};
use std::{
    io::Write,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
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

impl Pane {
    /// Spawn a new pane running `$SHELL` (or `/bin/sh` as fallback).
    pub fn new(id: usize, rows: u16, cols: u16) -> Result<Self> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
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
        PaneView { pane: self, focused }
    }

    /// Render screen content into an already-computed inner rect (no borders).
    pub fn render_content(&self, buf: &mut Buffer, inner: Rect) {
        if self.scroll.lock().unwrap().is_active() {
            self.render_scroll_content(buf, inner);
            return;
        }
        let parser = self.parser.lock().unwrap();
        let screen = parser.screen();
        let (rows, cols) = screen.size();

        for row in 0..rows.min(inner.height) {
            for col in 0..cols.min(inner.width) {
                let x = inner.x + col;
                let y = inner.y + row;

                if let Some(cell) = screen.cell(row, col) {
                    let content = cell.contents();
                    let text = if content.is_empty() { " " } else { &content };
                    let fg = vt100_color(cell.fgcolor());
                    let bg = vt100_color(cell.bgcolor());
                    let mut style = Style::default().fg(fg).bg(bg);
                    if cell.bold() {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    if cell.italic() {
                        style = style.add_modifier(Modifier::ITALIC);
                    }
                    if cell.underline() {
                        style = style.add_modifier(Modifier::UNDERLINED);
                    }
                    buf.set_string(x, y, text, style);
                } else {
                    buf.set_string(x, y, " ", Style::default());
                }
            }
        }

        // Render cursor as reversed cell
        let (cur_row, cur_col) = screen.cursor_position();
        let cx = inner.x + cur_col;
        let cy = inner.y + cur_row;
        if cx < inner.x + inner.width && cy < inner.y + inner.height {
            if let Some(cell) = screen.cell(cur_row, cur_col) {
                let content = cell.contents();
                let text = if content.is_empty() { " " } else { &content };
                buf.set_string(
                    cx,
                    cy,
                    text,
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White)
                        .add_modifier(Modifier::BOLD),
                );
            }
        }
    }
}

/// A renderable view of a pane, implementing ratatui's `Widget` trait.
pub struct PaneView<'a> {
    pane: &'a Pane,
    focused: bool,
}

impl Widget for PaneView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Border — active pane gets a yellow highlight
        let border_style = if self.focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(area);
        block.render(area, buf);

        self.pane.render_content(buf, inner);
    }
}

impl Pane {
    fn render_scroll_content(&self, buf: &mut Buffer, inner: Rect) {
        let scroll = self.scroll.lock().unwrap();
        let lines = scroll.visible_lines(inner.height as usize);
        for (row, line) in lines.iter().enumerate() {
            let y = inner.y + row as u16;
            if y >= inner.y + inner.height {
                break;
            }
            for (col, ch) in line.chars().take(inner.width as usize).enumerate() {
                buf.set_string(
                    inner.x + col as u16,
                    y,
                    ch.to_string(),
                    Style::default(),
                );
            }
        }
    }

    /// Capture a serializable snapshot of the current screen state.
    pub fn snapshot(&self) -> PaneSnapshot {
        if self.scroll.lock().unwrap().is_active() {
            return self.snapshot_scroll();
        }
        let parser = self.parser.lock().unwrap();
        let screen = parser.screen();
        let (rows, cols) = screen.size();
        let (cursor_row, cursor_col) = screen.cursor_position();
        let mut cells = Vec::with_capacity(rows as usize);
        for r in 0..rows {
            let mut row_cells = Vec::with_capacity(cols as usize);
            for c in 0..cols {
                if let Some(cell) = screen.cell(r, c) {
                    let contents = cell.contents();
                    let ch = contents.chars().next().unwrap_or(' ');
                    let (fg_r, fg_g, fg_b) = vt100_color_rgb(cell.fgcolor());
                    let (bg_r, bg_g, bg_b) = vt100_color_rgb(cell.bgcolor());
                    row_cells.push(CellData { ch, fg_r, fg_g, fg_b, bg_r, bg_g, bg_b, bold: cell.bold() });
                } else {
                    row_cells.push(CellData::default());
                }
            }
            cells.push(row_cells);
        }
        PaneSnapshot { id: self.id, rows, cols, cells, cursor_row, cursor_col }
    }

    fn snapshot_scroll(&self) -> PaneSnapshot {
        let scroll = self.scroll.lock().unwrap();
        let rows = 24u16;
        let cols = 80u16;
        let lines = scroll.visible_lines(rows as usize);
        let mut cells = Vec::with_capacity(rows as usize);
        for r in 0..rows {
            let mut row_cells = Vec::with_capacity(cols as usize);
            let line = lines.get(r as usize).copied().unwrap_or("");
            for c in 0..cols {
                let ch = line.chars().nth(c as usize).unwrap_or(' ');
                row_cells.push(CellData { ch, ..CellData::default() });
            }
            cells.push(row_cells);
        }
        PaneSnapshot {
            id: self.id,
            rows,
            cols,
            cells,
            cursor_row: 0,
            cursor_col: 0,
        }
    }
}  // end impl Pane (snapshot)

// ── Color conversion ──────────────────────────────────────────────────────────

fn vt100_color_rgb(c: vt100::Color) -> (u8, u8, u8) {
    match c {
        vt100::Color::Default => (0, 0, 0),
        vt100::Color::Idx(i) => indexed_to_rgb(i),
        vt100::Color::Rgb(r, g, b) => (r, g, b),
    }
}

fn indexed_to_rgb(idx: u8) -> (u8, u8, u8) {
    // Basic 16-color approximation; good enough for snapshot serialization
    match idx {
        0 => (0, 0, 0),
        1 => (128, 0, 0),
        2 => (0, 128, 0),
        3 => (128, 128, 0),
        4 => (0, 0, 128),
        5 => (128, 0, 128),
        6 => (0, 128, 128),
        7 => (192, 192, 192),
        8 => (128, 128, 128),
        9 => (255, 0, 0),
        10 => (0, 255, 0),
        11 => (255, 255, 0),
        12 => (0, 0, 255),
        13 => (255, 0, 255),
        14 => (0, 255, 255),
        15 => (255, 255, 255),
        _ => (128, 128, 128),
    }
}

fn vt100_color(c: vt100::Color) -> Color {
    match c {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(i) => Color::Indexed(i),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}
