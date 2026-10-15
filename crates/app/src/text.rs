//! The Apple II's 40 x 24 text page as the Monitor's COUT fills it, and the line editor that
//! stands in for GETLN.

pub const COLS: usize = 40;
pub const ROWS: usize = 24;

#[derive(Clone)]
pub struct TextPage {
    cells: [u8; COLS * ROWS],
    row: usize,
    col: usize,
    /// How many times the page has scrolled.
    scrolls: usize,
}

impl Default for TextPage {
    fn default() -> Self {
        TextPage {
            cells: [b' '; COLS * ROWS],
            row: ROWS - 1,
            col: 0,
            scrolls: 0,
        }
    }
}

impl TextPage {
    pub fn char_at(&self, row: usize, col: usize) -> char {
        self.cells[row * COLS + col] as char
    }

    pub fn cursor(&self) -> (usize, usize) {
        (self.row, self.col)
    }

    /// A character at (row, col), the cursor staying where it is.
    pub fn put_at(&mut self, row: usize, col: usize, c: char) {
        self.cells[row * COLS + col] = if c.is_ascii() {
            c.to_ascii_uppercase() as u8
        } else {
            b'?'
        };
    }

    pub fn set_cursor(&mut self, row: usize, col: usize) {
        (self.row, self.col) = (row, col);
    }

    /// HOME: clear the page, cursor at the top left.
    pub fn home(&mut self) {
        self.cells.fill(b' ');
        (self.row, self.col) = (0, 0);
    }

    /// COUT: `\n` is a carriage return; a character in column 40 goes on the next line, and
    /// a line past the bottom scrolls the page up.
    pub fn put(&mut self, c: char) {
        if c == '\n' {
            self.newline();
            return;
        }
        let b = if c.is_ascii() {
            c.to_ascii_uppercase() as u8
        } else {
            b'?'
        };
        self.cells[self.row * COLS + self.col] = b;
        self.col += 1;
        if self.col == COLS {
            self.newline();
        }
    }

    pub fn print(&mut self, text: &str) {
        text.chars().for_each(|c| self.put(c));
    }

    fn newline(&mut self) {
        self.col = 0;
        if self.row + 1 < ROWS {
            self.row += 1;
        } else {
            self.cells.copy_within(COLS.., 0);
            self.cells[COLS * (ROWS - 1)..].fill(b' ');
            self.scrolls += 1;
        }
    }
}

/// The line being typed: shown on a copy of the page as it was when the game asked, so it can
/// be edited anywhere (the original's GETLN only had backspace).
pub struct LineEditor {
    page: TextPage,
    line: Vec<char>,
    cursor: usize,
    history: Vec<String>,
    /// Which history entry is shown while browsing with the arrow keys.
    browsing: Option<usize>,
}

/// GETLN's limit.
const MAX_LINE: usize = 239;

impl LineEditor {
    pub fn new() -> Self {
        LineEditor {
            page: TextPage::default(),
            line: Vec::new(),
            cursor: 0,
            history: Vec::new(),
            browsing: None,
        }
    }

    /// Start a new line on `page` (the text so far, with the prompt).
    pub fn start(&mut self, page: &TextPage) {
        self.page = page.clone();
        self.line.clear();
        self.cursor = 0;
        self.browsing = None;
    }

    /// The page with the line typed so far, and where the cursor is on it.
    pub fn view(&self) -> (TextPage, (usize, usize)) {
        let mut page = self.page.clone();
        let mut at = None;
        for (i, &c) in self.line.iter().enumerate() {
            if i == self.cursor {
                at = Some((page.cursor(), page.scrolls));
            }
            page.put(c);
        }
        let ((row, col), scrolls) = at.unwrap_or((page.cursor(), page.scrolls));
        // The rest of the line may have scrolled the cursor's row up.
        let row = row.saturating_sub(page.scrolls - scrolls);
        (page, (row, col))
    }

    /// Typed text, upper-cased; only what the Apple's keyboard could type.
    pub fn insert(&mut self, text: &str) {
        for c in text.chars().map(|c| c.to_ascii_uppercase()) {
            if (' '..='_').contains(&c) && self.line.len() < MAX_LINE {
                self.line.insert(self.cursor, c);
                self.cursor += 1;
            }
        }
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.line.remove(self.cursor);
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.line.len() {
            self.line.remove(self.cursor);
        }
    }

    pub fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.line.len());
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.line.len();
    }

    pub fn clear(&mut self) {
        self.line.clear();
        self.cursor = 0;
    }

    /// The previous command (`up`) or the next one.
    pub fn browse(&mut self, up: bool) {
        if self.history.is_empty() {
            return;
        }
        let n = match (self.browsing, up) {
            (None, true) => Some(self.history.len() - 1),
            (None, false) => None,
            (Some(i), true) => Some(i.saturating_sub(1)),
            (Some(i), false) if i + 1 < self.history.len() => Some(i + 1),
            (Some(_), false) => None,
        };
        self.browsing = n;
        self.line = n.map_or_else(Vec::new, |i| self.history[i].chars().collect());
        self.cursor = self.line.len();
    }

    /// RETURN: the line, which also goes into the history; the page now ends with the line
    /// and the carriage return the Monitor echoes.
    pub fn enter(&mut self, remember: bool) -> (String, TextPage) {
        let line: String = self.line.iter().collect();
        if remember && !line.is_empty() && self.history.last() != Some(&line) {
            self.history.push(line.clone());
        }
        let mut page = self.page.clone();
        page.print(&line);
        page.put('\n');
        (line, page)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(p: &TextPage, r: usize) -> String {
        (0..COLS)
            .map(|c| p.char_at(r, c))
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    #[test]
    fn prints_scrolls_and_wraps_at_40() {
        let mut p = TextPage::default();
        p.print("ONE\nTWO");
        assert_eq!(row(&p, 22), "ONE");
        assert_eq!(row(&p, 23), "TWO");
        p.print("\n");
        p.print(&"X".repeat(40));
        assert_eq!(row(&p, 22), "X".repeat(40));
        assert_eq!(p.cursor(), (23, 0));
        p.home();
        assert_eq!((row(&p, 23), p.cursor()), (String::new(), (0, 0)));
    }

    #[test]
    fn edits_and_remembers_lines() {
        let mut e = LineEditor::new();
        let mut p = TextPage::default();
        p.print("ENTER COMMAND?");
        e.start(&p);
        e.insert("lok sink");
        e.home();
        e.right();
        e.right();
        e.insert("o");
        let (view, at) = e.view();
        assert_eq!(row(&view, 23), "ENTER COMMAND?LOOK SINK");
        assert_eq!(at, (23, 17));
        let (line, page) = e.enter(true);
        assert_eq!(line, "LOOK SINK");
        assert_eq!(row(&page, 22), "ENTER COMMAND?LOOK SINK");
        e.start(&page);
        e.browse(true);
        assert_eq!(e.enter(true).0, "LOOK SINK");
    }
}
