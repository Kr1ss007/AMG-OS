//! Terminal Grid Buffer
//!
//! Maintains character cell matrix, cursor position, and ANSI styling.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub character: char,
    pub foreground_color: u32,
    pub background_color: u32,
    pub is_bold: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            character: ' ',
            foreground_color: 0xE0E0E0,
            background_color: 0x121214,
            is_bold: false,
        }
    }
}

pub struct TerminalGrid {
    pub cols: usize,
    pub rows: usize,
    pub cursor_col: usize,
    pub cursor_row: usize,
    pub cells: Vec<Cell>,
}

impl TerminalGrid {
    pub fn new(cols: usize, rows: usize) -> Self {
        let total = cols * rows;
        Self {
            cols,
            rows,
            cursor_col: 0,
            cursor_row: 0,
            cells: vec![Cell::default(); total],
        }
    }

    pub fn write_char(&mut self, c: char) {
        match c {
            '\n' => {
                self.cursor_col = 0;
                if self.cursor_row + 1 < self.rows {
                    self.cursor_row += 1;
                } else {
                    self.scroll_up();
                }
            }
            '\r' => {
                self.cursor_col = 0;
            }
            _ => {
                if self.cursor_col >= self.cols {
                    self.cursor_col = 0;
                    if self.cursor_row + 1 < self.rows {
                        self.cursor_row += 1;
                    } else {
                        self.scroll_up();
                    }
                }
                let idx = self.cursor_row * self.cols + self.cursor_col;
                if idx < self.cells.len() {
                    self.cells[idx].character = c;
                    self.cursor_col += 1;
                }
            }
        }
    }

    fn scroll_up(&mut self) {
        if self.rows > 1 {
            self.cells.drain(0..self.cols);
            self.cells.extend(vec![Cell::default(); self.cols]);
        }
    }
}
