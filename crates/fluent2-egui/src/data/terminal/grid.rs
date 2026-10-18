// Adapted from TheDancingDeveloper-org/rdpapp migration-tool/src/ssh/terminal.rs @ 4dff7e2.
// Copyright (c) 2026 The rdpapp authors. Licensed MIT OR Apache-2.0.

//! The terminal grid model: a `vte` parser driving a cell grid with scrollback.
//!
//! No egui here. The grid keeps colours as [`TermColor`] (default, palette index or RGB) so the
//! view maps the 16 ANSI colours to theme tokens at paint time.

use std::collections::VecDeque;

use unicode_width::UnicodeWidthChar;
use vte::{Params, Parser, Perform};

/// Default scrollback length, in lines.
pub const DEFAULT_SCROLLBACK: usize = 5000;

/// A cell colour as the program asked for it.
///
/// ```
/// use fluent2_egui::data::terminal::TermColor;
/// assert_eq!(TermColor::default(), TermColor::Default);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TermColor {
    /// The terminal's default foreground or background.
    #[default]
    Default,
    /// A 256-colour palette index. 0 to 15 are the ANSI colours, which the view takes from the
    /// theme.
    Indexed(u8),
    /// A 24-bit colour (`SGR 38;2` / `48;2`).
    Rgb(u8, u8, u8),
}

/// One character cell.
///
/// ```
/// let cell = fluent2_egui::data::terminal::Cell::default();
/// assert_eq!(cell.ch, ' ');
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    /// The character. A wide character's second cell holds `' '` with [`Cell::spacer`] set.
    pub ch: char,
    /// Foreground colour.
    pub fg: TermColor,
    /// Background colour.
    pub bg: TermColor,
    /// Bold (`SGR 1`).
    pub bold: bool,
    /// Italic (`SGR 3`).
    pub italic: bool,
    /// Underline (`SGR 4`).
    pub underline: bool,
    /// Inverse video (`SGR 7`): the view swaps foreground and background.
    pub inverse: bool,
    /// The right half of a double-width character.
    pub spacer: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: TermColor::Default,
            bg: TermColor::Default,
            bold: false,
            italic: false,
            underline: false,
            inverse: false,
            spacer: false,
        }
    }
}

/// A position in the buffer: a stable line number and a column.
///
/// Line numbers count every line the buffer has held, so they stay valid while new output
/// scrolls in. A line evicted from scrollback is no longer addressable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GridPos {
    /// Stable line number.
    pub line: u64,
    /// Column, from 0.
    pub col: usize,
}

/// A search match or link: columns `start..end` of one line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    /// Stable line number.
    pub line: u64,
    /// First column.
    pub start: usize,
    /// One past the last column.
    pub end: usize,
}

#[derive(Clone, Copy, Default)]
struct Pen {
    fg: TermColor,
    bg: TermColor,
    bold: bool,
    italic: bool,
    underline: bool,
    inverse: bool,
}

impl Pen {
    fn cell(self, ch: char) -> Cell {
        Cell {
            ch,
            fg: self.fg,
            bg: self.bg,
            bold: self.bold,
            italic: self.italic,
            underline: self.underline,
            inverse: self.inverse,
            spacer: false,
        }
    }
}

/// Grid state that implements [`Perform`]. Kept apart from the parser so `advance` can borrow
/// both.
struct State {
    cols: usize,
    rows: usize,
    cells: Vec<Vec<Cell>>,
    alt_cells: Vec<Vec<Cell>>,
    history: VecDeque<Vec<Cell>>,
    history_max: usize,
    /// Lines dropped from the front of history; the stable number of `history[0]`.
    evicted: u64,
    scroll_offset: usize,
    cursor_row: usize,
    cursor_col: usize,
    /// A character was written in the last column; the next one wraps first.
    wrap_pending: bool,
    saved_row: usize,
    saved_col: usize,
    using_alt: bool,
    scroll_top: usize,
    scroll_bottom: usize,
    cursor_visible: bool,
    pen: Pen,
    bell: bool,
    title: Option<String>,
}

fn blank_row(cols: usize) -> Vec<Cell> {
    vec![Cell::default(); cols]
}

impl State {
    fn new(cols: usize, rows: usize, history_max: usize) -> Self {
        let cols = cols.max(1);
        let rows = rows.max(1);
        Self {
            cols,
            rows,
            cells: vec![blank_row(cols); rows],
            alt_cells: vec![blank_row(cols); rows],
            history: VecDeque::new(),
            history_max,
            evicted: 0,
            scroll_offset: 0,
            cursor_row: 0,
            cursor_col: 0,
            wrap_pending: false,
            saved_row: 0,
            saved_col: 0,
            using_alt: false,
            scroll_top: 0,
            scroll_bottom: rows - 1,
            cursor_visible: true,
            pen: Pen::default(),
            bell: false,
            title: None,
        }
    }

    fn set_cell(&mut self, row: usize, col: usize, cell: Cell) {
        if let Some(c) = self.cells.get_mut(row).and_then(|r| r.get_mut(col)) {
            *c = cell;
        }
    }

    fn clear_cols(&mut self, row: usize, from: usize, to: usize) {
        if let Some(r) = self.cells.get_mut(row) {
            let to = to.min(r.len());
            for c in r.iter_mut().take(to).skip(from) {
                *c = Cell::default();
            }
        }
    }

    fn clamp_cursor(&mut self) {
        self.cursor_row = self.cursor_row.min(self.rows - 1);
        self.cursor_col = self.cursor_col.min(self.cols - 1);
        self.wrap_pending = false;
    }

    fn scroll_up(&mut self) {
        let full = self.scroll_top == 0 && self.scroll_bottom == self.rows - 1;
        if full && !self.using_alt {
            let row = if self.cells.is_empty() {
                blank_row(self.cols)
            } else {
                self.cells.remove(0)
            };
            self.history.push_back(row);
            if self.history.len() > self.history_max {
                self.history.pop_front();
                self.evicted += 1;
            }
            self.cells.push(blank_row(self.cols));
            if self.scroll_offset > 0 {
                self.scroll_offset = (self.scroll_offset + 1).min(self.history.len());
            }
        } else {
            let top = self.scroll_top;
            let bot = (self.scroll_bottom + 1).min(self.cells.len());
            if top < bot {
                self.cells.remove(top);
                self.cells.insert(bot - 1, blank_row(self.cols));
            }
        }
    }

    fn scroll_down(&mut self) {
        let top = self.scroll_top;
        let bot = (self.scroll_bottom + 1).min(self.cells.len());
        if top < bot {
            self.cells.remove(bot - 1);
            self.cells.insert(top, blank_row(self.cols));
        }
    }

    fn line_feed(&mut self) {
        self.wrap_pending = false;
        if self.cursor_row == self.scroll_bottom {
            self.scroll_up();
        } else {
            self.cursor_row = (self.cursor_row + 1).min(self.rows - 1);
        }
    }

    fn put_char(&mut self, ch: char) {
        let width = ch.width().unwrap_or(0);
        if width == 0 {
            // Combining marks and other zero-width characters are not drawn.
            return;
        }
        if self.wrap_pending || self.cursor_col + width > self.cols {
            self.cursor_col = 0;
            self.line_feed();
        }
        let (row, col) = (self.cursor_row, self.cursor_col);
        self.set_cell(row, col, self.pen.cell(ch));
        if width == 2 {
            let mut spacer = self.pen.cell(' ');
            spacer.spacer = true;
            self.set_cell(row, col + 1, spacer);
        }
        let next = col + width;
        if next >= self.cols {
            self.cursor_col = self.cols - 1;
            self.wrap_pending = true;
        } else {
            self.cursor_col = next;
        }
    }

    fn switch_to_alt(&mut self) {
        if self.using_alt {
            return;
        }
        self.saved_row = self.cursor_row;
        self.saved_col = self.cursor_col;
        std::mem::swap(&mut self.cells, &mut self.alt_cells);
        for row in &mut self.cells {
            *row = blank_row(self.cols);
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.wrap_pending = false;
        self.scroll_offset = 0;
        self.scroll_top = 0;
        self.scroll_bottom = self.rows - 1;
        self.using_alt = true;
    }

    fn switch_to_main(&mut self) {
        if !self.using_alt {
            return;
        }
        std::mem::swap(&mut self.cells, &mut self.alt_cells);
        self.cursor_row = self.saved_row;
        self.cursor_col = self.saved_col;
        self.clamp_cursor();
        self.scroll_offset = 0;
        self.scroll_top = 0;
        self.scroll_bottom = self.rows - 1;
        self.using_alt = false;
    }

    fn handle_sgr(&mut self, params: &Params) {
        let mut iter = params.iter();
        if params.is_empty() {
            self.pen = Pen::default();
            return;
        }
        while let Some(p) = iter.next() {
            let code = p.first().copied().unwrap_or(0);
            match code {
                0 => self.pen = Pen::default(),
                1 => self.pen.bold = true,
                3 => self.pen.italic = true,
                4 => self.pen.underline = true,
                7 => self.pen.inverse = true,
                22 => self.pen.bold = false,
                23 => self.pen.italic = false,
                24 => self.pen.underline = false,
                27 => self.pen.inverse = false,
                30..=37 => self.pen.fg = TermColor::Indexed((code - 30) as u8),
                39 => self.pen.fg = TermColor::Default,
                40..=47 => self.pen.bg = TermColor::Indexed((code - 40) as u8),
                49 => self.pen.bg = TermColor::Default,
                90..=97 => self.pen.fg = TermColor::Indexed((code - 90 + 8) as u8),
                100..=107 => self.pen.bg = TermColor::Indexed((code - 100 + 8) as u8),
                38 | 48 => {
                    // Colon form (`38:2:r:g:b`) arrives as one sub-parameter list; the
                    // semicolon form spreads over the following parameters.
                    let color = if p.len() > 1 {
                        extended_color(p.iter().skip(1).copied())
                    } else {
                        extended_color_from(&mut iter)
                    };
                    if let Some(color) = color {
                        if code == 38 {
                            self.pen.fg = color;
                        } else {
                            self.pen.bg = color;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn param(params: &Params, n: usize, default: u16) -> usize {
        let v = params.iter().nth(n).and_then(|p| p.first().copied()).unwrap_or(0);
        usize::from(if v == 0 { default } else { v })
    }
}

fn to_u8(v: u16) -> u8 {
    u8::try_from(v).unwrap_or(u8::MAX)
}

fn extended_color(mut it: impl Iterator<Item = u16>) -> Option<TermColor> {
    match it.next()? {
        5 => Some(TermColor::Indexed(to_u8(it.next()?))),
        2 => {
            let mut rest: Vec<u16> = it.collect();
            // `38:2:<colour space>:r:g:b` has an extra leading id.
            if rest.len() >= 4 {
                rest.remove(0);
            }
            let r = to_u8(*rest.first()?);
            let g = to_u8(*rest.get(1)?);
            let b = to_u8(*rest.get(2)?);
            Some(TermColor::Rgb(r, g, b))
        }
        _ => None,
    }
}

fn extended_color_from<'a>(iter: &mut impl Iterator<Item = &'a [u16]>) -> Option<TermColor> {
    let mut next = || iter.next().and_then(|p| p.first().copied());
    match next()? {
        5 => Some(TermColor::Indexed(to_u8(next()?))),
        2 => {
            let r = to_u8(next()?);
            let g = to_u8(next()?);
            let b = to_u8(next()?);
            Some(TermColor::Rgb(r, g, b))
        }
        _ => None,
    }
}

impl Perform for State {
    fn print(&mut self, c: char) {
        self.put_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | 0x0b | 0x0c => self.line_feed(),
            b'\r' => {
                self.cursor_col = 0;
                self.wrap_pending = false;
            }
            0x08 => {
                self.cursor_col = self.cursor_col.saturating_sub(1);
                self.wrap_pending = false;
            }
            b'\t' => {
                let next = (self.cursor_col / 8 + 1) * 8;
                self.cursor_col = next.min(self.cols - 1);
            }
            0x07 => self.bell = true,
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, action: char) {
        let p = |n: usize, d: u16| Self::param(params, n, d);

        if intermediates == b"?" {
            let on = match action {
                'h' => true,
                'l' => false,
                _ => return,
            };
            for sub in params.iter() {
                match sub.first().copied().unwrap_or(0) {
                    47 | 1047 | 1049 if on => self.switch_to_alt(),
                    47 | 1047 | 1049 => self.switch_to_main(),
                    25 => self.cursor_visible = on,
                    _ => {}
                }
            }
            return;
        }
        if !intermediates.is_empty() {
            return;
        }

        let rows = self.rows;
        let cols = self.cols;
        match action {
            'H' | 'f' => {
                self.cursor_row = p(0, 1) - 1;
                self.cursor_col = p(1, 1) - 1;
                self.clamp_cursor();
            }
            'A' => {
                self.cursor_row = self.cursor_row.saturating_sub(p(0, 1));
                self.wrap_pending = false;
            }
            'B' | 'e' => {
                self.cursor_row = self.cursor_row.saturating_add(p(0, 1));
                self.clamp_cursor();
            }
            'C' | 'a' => {
                self.cursor_col = self.cursor_col.saturating_add(p(0, 1));
                self.clamp_cursor();
            }
            'D' => {
                self.cursor_col = self.cursor_col.saturating_sub(p(0, 1));
                self.wrap_pending = false;
            }
            'E' => {
                self.cursor_row = self.cursor_row.saturating_add(p(0, 1));
                self.cursor_col = 0;
                self.clamp_cursor();
            }
            'F' => {
                self.cursor_row = self.cursor_row.saturating_sub(p(0, 1));
                self.cursor_col = 0;
                self.wrap_pending = false;
            }
            'G' | '`' => {
                self.cursor_col = p(0, 1) - 1;
                self.clamp_cursor();
            }
            'd' => {
                self.cursor_row = p(0, 1) - 1;
                self.clamp_cursor();
            }
            'J' => {
                let (row, col) = (self.cursor_row, self.cursor_col);
                match Self::param(params, 0, 0) {
                    0 => {
                        self.clear_cols(row, col, cols);
                        for r in self.cells.iter_mut().skip(row + 1) {
                            *r = blank_row(cols);
                        }
                    }
                    1 => {
                        for r in self.cells.iter_mut().take(row) {
                            *r = blank_row(cols);
                        }
                        self.clear_cols(row, 0, col + 1);
                    }
                    2 | 3 => {
                        for r in &mut self.cells {
                            *r = blank_row(cols);
                        }
                    }
                    _ => {}
                }
            }
            'K' => {
                let (row, col) = (self.cursor_row, self.cursor_col);
                match Self::param(params, 0, 0) {
                    0 => self.clear_cols(row, col, cols),
                    1 => self.clear_cols(row, 0, col + 1),
                    2 => self.clear_cols(row, 0, cols),
                    _ => {}
                }
            }
            'X' => {
                let (row, col) = (self.cursor_row, self.cursor_col);
                self.clear_cols(row, col, col.saturating_add(p(0, 1)));
            }
            'P' => {
                let n = p(0, 1);
                let col = self.cursor_col;
                if let Some(row) = self.cells.get_mut(self.cursor_row)
                    && col < row.len()
                {
                    let n = n.min(row.len() - col);
                    row.drain(col..col + n);
                    row.resize(cols, Cell::default());
                }
            }
            '@' => {
                let n = p(0, 1);
                let col = self.cursor_col;
                if let Some(row) = self.cells.get_mut(self.cursor_row)
                    && col < row.len()
                {
                    let n = n.min(row.len() - col);
                    for _ in 0..n {
                        row.insert(col, Cell::default());
                    }
                    row.truncate(cols);
                }
            }
            'S' => {
                for _ in 0..p(0, 1).min(rows) {
                    self.scroll_up();
                }
            }
            'T' => {
                for _ in 0..p(0, 1).min(rows) {
                    self.scroll_down();
                }
            }
            'L' | 'M' => {
                if self.cursor_row < self.scroll_top || self.cursor_row > self.scroll_bottom {
                    return;
                }
                let top = self.cursor_row;
                let bot = (self.scroll_bottom + 1).min(self.cells.len());
                for _ in 0..p(0, 1).min(rows) {
                    if top < bot {
                        if action == 'L' {
                            self.cells.remove(bot - 1);
                            self.cells.insert(top, blank_row(cols));
                        } else {
                            self.cells.remove(top);
                            self.cells.insert(bot - 1, blank_row(cols));
                        }
                    }
                }
                self.cursor_col = 0;
                self.wrap_pending = false;
            }
            'r' => {
                let top = p(0, 1) - 1;
                let bot = Self::param(params, 1, u16::try_from(rows).unwrap_or(u16::MAX)) - 1;
                let bot = bot.min(rows - 1);
                if top < bot {
                    self.scroll_top = top;
                    self.scroll_bottom = bot;
                } else {
                    self.scroll_top = 0;
                    self.scroll_bottom = rows - 1;
                }
                self.cursor_row = 0;
                self.cursor_col = 0;
                self.wrap_pending = false;
            }
            'm' => self.handle_sgr(params),
            _ => {}
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if let [kind, title, ..] = params
            && (*kind == b"0" || *kind == b"2")
            && !title.is_empty()
        {
            self.title = Some(String::from_utf8_lossy(title).into_owned());
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        if !intermediates.is_empty() {
            return;
        }
        match byte {
            b'D' => self.line_feed(),
            b'E' => {
                self.cursor_col = 0;
                self.line_feed();
            }
            b'M' => {
                self.wrap_pending = false;
                if self.cursor_row == self.scroll_top {
                    self.scroll_down();
                } else {
                    self.cursor_row = self.cursor_row.saturating_sub(1);
                }
            }
            b'7' => {
                self.saved_row = self.cursor_row;
                self.saved_col = self.cursor_col;
            }
            b'8' => {
                self.cursor_row = self.saved_row;
                self.cursor_col = self.saved_col;
                self.clamp_cursor();
            }
            b'c' => {
                let history_max = self.history_max;
                let (cols, rows) = (self.cols, self.rows);
                let history = std::mem::take(&mut self.history);
                let evicted = self.evicted;
                *self = Self::new(cols, rows, history_max);
                self.history = history;
                self.evicted = evicted;
            }
            _ => {}
        }
    }
}

/// A VT terminal buffer: feed it the bytes a program writes, read back cells.
///
/// The grid understands the common xterm subset (cursor movement, erase, insert and delete,
/// scrolling regions, the alternate screen, SGR colours including 256-colour and 24-bit, the
/// OSC window title and BEL). It does not write anything back: input and PTY transport stay
/// with the app.
///
/// ```
/// use fluent2_egui::data::terminal::TerminalGrid;
///
/// let mut grid = TerminalGrid::new(20, 3);
/// grid.process(b"hello \x1b[1;31mworld\x1b[0m\r\nnext");
/// assert_eq!(grid.visible_text(), "hello world\nnext\n");
/// ```
pub struct TerminalGrid {
    parser: Parser,
    state: State,
    generation: u64,
}

impl std::fmt::Debug for TerminalGrid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalGrid")
            .field("cols", &self.state.cols)
            .field("rows", &self.state.rows)
            .field("history", &self.state.history.len())
            .finish_non_exhaustive()
    }
}

impl Default for TerminalGrid {
    fn default() -> Self {
        Self::new(80, 24)
    }
}

impl TerminalGrid {
    /// A blank `cols` × `rows` grid with [`DEFAULT_SCROLLBACK`] lines of history. Zero sizes
    /// are raised to 1.
    #[must_use]
    pub fn new(cols: usize, rows: usize) -> Self {
        Self::with_scrollback(cols, rows, DEFAULT_SCROLLBACK)
    }

    /// A blank grid that keeps at most `scrollback` lines of history.
    #[must_use]
    pub fn with_scrollback(cols: usize, rows: usize, scrollback: usize) -> Self {
        Self {
            parser: Parser::new(),
            state: State::new(cols, rows, scrollback),
            generation: 0,
        }
    }

    /// Feed output bytes. Partial escape sequences and UTF-8 carry over to the next call.
    pub fn process(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.state, bytes);
        self.generation = self.generation.wrapping_add(1);
    }

    /// A counter that changes whenever the content changes (output, resize, clear). Views use
    /// it to cache work such as search matches.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Columns.
    #[must_use]
    pub fn cols(&self) -> usize {
        self.state.cols
    }

    /// Rows on screen.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.state.rows
    }

    /// Resize the screen. Rows and columns are cut or padded; the cursor is clamped. Zero sizes
    /// are raised to 1. Reflow is not attempted.
    pub fn resize(&mut self, cols: usize, rows: usize) {
        let s = &mut self.state;
        let cols = cols.max(1);
        let rows = rows.max(1);
        if cols == s.cols && rows == s.rows {
            return;
        }
        self.generation = self.generation.wrapping_add(1);
        // Shrinking keeps the bottom of the screen (where the cursor usually is) by pushing the
        // top rows into history.
        while s.cells.len() > rows && s.cursor_row > 0 {
            let row = s.cells.remove(0);
            if !s.using_alt {
                s.history.push_back(row);
            }
            s.cursor_row -= 1;
        }
        s.cells.resize(rows, blank_row(cols));
        s.alt_cells.resize(rows, blank_row(cols));
        for row in s.cells.iter_mut().chain(s.alt_cells.iter_mut()) {
            row.resize(cols, Cell::default());
        }
        while s.history.len() > s.history_max {
            s.history.pop_front();
            s.evicted += 1;
        }
        s.cols = cols;
        s.rows = rows;
        s.scroll_top = 0;
        s.scroll_bottom = rows - 1;
        s.scroll_offset = s.scroll_offset.min(s.history.len());
        s.clamp_cursor();
    }

    /// Clear the screen, history and modes (like `reset`), keeping the size.
    pub fn clear(&mut self) {
        self.state = State::new(self.state.cols, self.state.rows, self.state.history_max);
        self.parser = Parser::new();
        self.generation = self.generation.wrapping_add(1);
    }

    /// Cursor `(row, col)` on screen.
    #[must_use]
    pub fn cursor(&self) -> (usize, usize) {
        (self.state.cursor_row, self.state.cursor_col)
    }

    /// Whether the program shows the cursor (`DECTCEM`, `CSI ?25h/l`).
    #[must_use]
    pub fn cursor_visible(&self) -> bool {
        self.state.cursor_visible
    }

    /// Whether the alternate screen is active.
    #[must_use]
    pub fn alt_screen(&self) -> bool {
        self.state.using_alt
    }

    /// Scrolling region `(top, bottom)`, inclusive screen rows.
    #[must_use]
    pub fn scroll_region(&self) -> (usize, usize) {
        (self.state.scroll_top, self.state.scroll_bottom)
    }

    /// `true` once after a BEL was received.
    pub fn take_bell(&mut self) -> bool {
        std::mem::take(&mut self.state.bell)
    }

    /// The window title set by `OSC 0` or `OSC 2` since the last call.
    pub fn take_title(&mut self) -> Option<String> {
        self.state.title.take()
    }

    /// Lines of scrollback.
    #[must_use]
    pub fn history_len(&self) -> usize {
        self.state.history.len()
    }

    /// Lines scrolled up into history (0 is the live screen).
    #[must_use]
    pub fn scroll_offset(&self) -> usize {
        self.state.scroll_offset
    }

    /// Scroll by `delta` lines: positive goes up into history. Clamped.
    pub fn scroll_by(&mut self, delta: isize) {
        let max = self.state.history.len();
        let next = if delta >= 0 {
            self.state.scroll_offset.saturating_add(delta.unsigned_abs())
        } else {
            self.state.scroll_offset.saturating_sub(delta.unsigned_abs())
        };
        self.state.scroll_offset = next.min(max);
    }

    /// Jump back to the live screen.
    pub fn scroll_to_bottom(&mut self) {
        self.state.scroll_offset = 0;
    }

    /// Stable line number of the first line still held (oldest history line).
    #[must_use]
    pub fn first_line(&self) -> u64 {
        self.state.evicted
    }

    /// Stable line number one past the last screen row.
    #[must_use]
    pub fn end_line(&self) -> u64 {
        self.state.evicted + (self.state.history.len() + self.state.cells.len()) as u64
    }

    /// Stable line number of the top row of the view at the current scroll offset.
    #[must_use]
    pub fn view_top(&self) -> u64 {
        self.state.evicted
            + (self.state.history.len() - self.state.scroll_offset.min(self.state.history.len())) as u64
    }

    /// Stable line number of screen row 0 on the live screen.
    #[must_use]
    pub fn screen_top(&self) -> u64 {
        self.state.evicted + self.state.history.len() as u64
    }

    /// The cells of a line, or `None` when it was evicted or does not exist yet.
    #[must_use]
    pub fn line(&self, line: u64) -> Option<&[Cell]> {
        let idx = usize::try_from(line.checked_sub(self.state.evicted)?).ok()?;
        let h = self.state.history.len();
        if idx < h {
            self.state.history.get(idx).map(Vec::as_slice)
        } else {
            self.state.cells.get(idx - h).map(Vec::as_slice)
        }
    }

    /// Text of a line with trailing blanks removed.
    #[must_use]
    pub fn line_text(&self, line: u64) -> String {
        self.line(line).map(row_text).unwrap_or_default()
    }

    /// The rows in view, top to bottom, at the current scroll offset.
    #[must_use]
    pub fn view_text(&self) -> String {
        let top = self.view_top();
        lines_text(self, top, top + self.state.rows as u64)
    }

    /// The live screen as text, rows joined by `\n`, trailing blanks removed per row.
    #[must_use]
    pub fn visible_text(&self) -> String {
        let top = self.screen_top();
        lines_text(self, top, top + self.state.rows as u64)
    }

    /// History and screen as text.
    #[must_use]
    pub fn buffer_text(&self) -> String {
        lines_text(self, self.first_line(), self.end_line())
    }

    /// Text between two positions (inclusive of both), as a selection copies it.
    #[must_use]
    pub fn text_between(&self, a: GridPos, b: GridPos) -> String {
        let (start, end) = if a <= b { (a, b) } else { (b, a) };
        let mut out = String::new();
        let mut line = start.line.max(self.first_line());
        while line <= end.line && line < self.end_line() {
            if let Some(cells) = self.line(line) {
                let from = if line == start.line { start.col } else { 0 };
                let to = if line == end.line {
                    end.col.saturating_add(1)
                } else {
                    cells.len()
                };
                let to = to.min(cells.len());
                let text: String = cells
                    .get(from.min(to)..to)
                    .unwrap_or(&[])
                    .iter()
                    .filter(|c| !c.spacer)
                    .map(|c| c.ch)
                    .collect();
                out.push_str(text.trim_end());
            }
            if line != end.line {
                out.push('\n');
            }
            line += 1;
        }
        out
    }

    /// Case-insensitive matches of `query` in history and screen, oldest first. Matches do not
    /// span lines. An empty or blank query has no matches.
    #[must_use]
    pub fn find(&self, query: &str) -> Vec<Span> {
        let needle: Vec<char> = query.trim().chars().flat_map(char::to_lowercase).collect();
        if needle.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for line in self.first_line()..self.end_line() {
            let Some(cells) = self.line(line) else { continue };
            let hay: Vec<char> = cells
                .iter()
                .map(|c| c.ch.to_lowercase().next().unwrap_or(c.ch))
                .collect();
            let mut i = 0;
            while i + needle.len() <= hay.len() {
                if hay.get(i..i + needle.len()) == Some(needle.as_slice()) {
                    out.push(Span {
                        line,
                        start: i,
                        end: i + needle.len(),
                    });
                    i += needle.len();
                } else {
                    i += 1;
                }
            }
        }
        out
    }

    /// `http://`, `https://` and `file://` links on a line.
    #[must_use]
    pub fn links(&self, line: u64) -> Vec<(Span, String)> {
        let Some(cells) = self.line(line) else {
            return Vec::new();
        };
        let chars: Vec<char> = cells.iter().map(|c| c.ch).collect();
        let mut out = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            let rest = chars.get(i..).unwrap_or(&[]);
            let scheme = ["https://", "http://", "file://"]
                .into_iter()
                .find(|s| starts_with(rest, s));
            if let Some(scheme) = scheme {
                let mut end = i + scheme.chars().count();
                while end < chars.len() && is_url_char(chars.get(end).copied().unwrap_or(' ')) {
                    end += 1;
                }
                // Trailing punctuation is usually the sentence, not the URL.
                while end > i
                    && matches!(
                        chars.get(end - 1),
                        Some('.' | ',' | ';' | ':' | ')' | ']' | '\'' | '"')
                    )
                {
                    end -= 1;
                }
                let url: String = chars.get(i..end).unwrap_or(&[]).iter().collect();
                if url.chars().count() > scheme.chars().count() {
                    out.push((Span { line, start: i, end }, url));
                }
                i = end.max(i + 1);
            } else {
                i += 1;
            }
        }
        out
    }
}

fn starts_with(chars: &[char], prefix: &str) -> bool {
    let mut it = chars.iter();
    prefix
        .chars()
        .all(|p| it.next().is_some_and(|c| c.eq_ignore_ascii_case(&p)))
}

fn is_url_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "-._~:/?#[]@!$&'()*+,;=%".contains(c)
}

fn row_text(row: &[Cell]) -> String {
    let s: String = row.iter().filter(|c| !c.spacer).map(|c| c.ch).collect();
    s.trim_end().to_string()
}

fn lines_text(grid: &TerminalGrid, from: u64, to: u64) -> String {
    (from..to)
        .map(|l| grid.line_text(l))
        .collect::<Vec<_>>()
        .join("\n")
}

/// RGB for a 256-colour palette index from 16 up (the 6×6×6 cube and the grey ramp). The view
/// takes 0 to 15 from the theme; for those this returns the xterm defaults.
///
/// ```
/// use fluent2_egui::data::terminal::color256;
/// assert_eq!(color256(16), (0, 0, 0));
/// assert_eq!(color256(231), (255, 255, 255));
/// assert_eq!(color256(232), (8, 8, 8));
/// ```
#[must_use]
pub fn color256(n: u8) -> (u8, u8, u8) {
    const C16: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (205, 0, 0),
        (0, 205, 0),
        (205, 205, 0),
        (0, 0, 238),
        (205, 0, 205),
        (0, 205, 205),
        (229, 229, 229),
        (127, 127, 127),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (92, 92, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];
    if n < 16 {
        return C16.get(usize::from(n)).copied().unwrap_or((0, 0, 0));
    }
    if n >= 232 {
        let v = 8 + (n - 232) * 10;
        return (v, v, v);
    }
    let n = n - 16;
    let s = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
    (s(n / 36), s((n / 6) % 6), s(n % 6))
}
