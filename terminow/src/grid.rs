//! Terminal Grid Buffer
//!
//! Maintains character cell matrix, cursor position, ANSI escape handling,
//! and screen text generation for cosmic-text shaping and WGPU rendering.

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
    escape_buf: Vec<u8>,
    in_escape: bool,
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
            escape_buf: Vec::new(),
            in_escape: false,
        }
    }

    /// Process raw byte stream from PTY
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            if self.in_escape {
                self.escape_buf.push(b);
                // Check for end of ANSI sequence
                if (b >= b'A' && b <= b'Z') || (b >= b'a' && b <= b'z') || b == b'~' {
                    self.process_escape_sequence();
                    self.in_escape = false;
                    self.escape_buf.clear();
                } else if self.escape_buf.len() > 64 {
                    // Abort runaway sequence
                    self.in_escape = false;
                    self.escape_buf.clear();
                }
                continue;
            }

            match b {
                0x1B => {
                    self.in_escape = true;
                    self.escape_buf.clear();
                    self.escape_buf.push(b);
                }
                0x08 | 0x7F => {
                    // Backspace
                    if self.cursor_col > 0 {
                        self.cursor_col -= 1;
                        let idx = self.cursor_row * self.cols + self.cursor_col;
                        if idx < self.cells.len() {
                            self.cells[idx] = Cell::default();
                        }
                    }
                }
                b'\t' => {
                    // Advance to next 8-space tab stop
                    let next_tab = (self.cursor_col + 8) & !7;
                    self.cursor_col = next_tab.min(self.cols.saturating_sub(1));
                }
                b'\r' => {
                    self.cursor_col = 0;
                }
                b'\n' => {
                    self.cursor_col = 0;
                    if self.cursor_row + 1 < self.rows {
                        self.cursor_row += 1;
                    } else {
                        self.scroll_up();
                    }
                }
                0x07 => {
                    // Bell - ignore or trigger audio beep
                }
                b => {
                    if b >= 0x20 {
                        self.write_char(b as char);
                    }
                }
            }
        }
    }

    pub fn write_char(&mut self, c: char) {
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
            self.cells[idx] = Cell {
                character: c,
                foreground_color: 0xE0E0E0,
                background_color: 0x121214,
                is_bold: false,
            };
            self.cursor_col += 1;
        }
    }

    fn process_escape_sequence(&mut self) {
        let seq = String::from_utf8_lossy(&self.escape_buf);
        if seq.starts_with("\x1B[") {
            let cmd = seq.chars().last().unwrap_or('\0');
            let params = &seq[2..seq.len().saturating_sub(1)];

            match cmd {
                'H' | 'f' => {
                    // Cursor Position
                    let parts: Vec<&str> = params.split(';').collect();
                    let r = parts.get(0).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
                    let c = parts.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
                    self.cursor_row = (r.saturating_sub(1)).min(self.rows.saturating_sub(1));
                    self.cursor_col = (c.saturating_sub(1)).min(self.cols.saturating_sub(1));
                }
                'J' => {
                    // Erase in Display
                    if params == "2" || params == "3" {
                        self.clear_all();
                        self.cursor_col = 0;
                        self.cursor_row = 0;
                    }
                }
                'K' => {
                    // Erase in Line
                    let start = self.cursor_row * self.cols + self.cursor_col;
                    let end = (self.cursor_row + 1) * self.cols;
                    for idx in start..end.min(self.cells.len()) {
                        self.cells[idx] = Cell::default();
                    }
                }
                _ => {}
            }
        }
    }

    pub fn clear_all(&mut self) {
        for cell in self.cells.iter_mut() {
            *cell = Cell::default();
        }
    }

    fn scroll_up(&mut self) {
        if self.rows > 1 {
            self.cells.drain(0..self.cols);
            self.cells.extend(vec![Cell::default(); self.cols]);
        }
    }

    /// Render grid content as full text string for cosmic-text shaping
    pub fn formatted_content(&self) -> String {
        let mut out = String::with_capacity(self.rows * (self.cols + 1));
        for r in 0..self.rows {
            let start = r * self.cols;
            let end = start + self.cols;
            if end <= self.cells.len() {
                let row_chars: String = self.cells[start..end].iter().map(|c| c.character).collect();
                out.push_str(row_chars.trim_end());
            }
            if r + 1 < self.rows {
                out.push('\n');
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_write_and_scroll() {
        let mut grid = TerminalGrid::new(10, 3);
        grid.write_bytes(b"hello\nworld\nline3\nline4");
        let content = grid.formatted_content();
        assert!(content.contains("world"));
        assert!(content.contains("line4"));
        assert!(!content.contains("hello")); // scrolled off
    }

    #[test]
    fn test_ansi_clear_sequence() {
        let mut grid = TerminalGrid::new(20, 5);
        grid.write_bytes(b"some text\x1B[2J");
        assert_eq!(grid.cells.iter().all(|c| c.character == ' '), true);
    }
}
