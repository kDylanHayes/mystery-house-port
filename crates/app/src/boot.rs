//! HELLO and MYSTERY.HELLO: the boot timeline from `gamedata.json`, played onto the text page
//! and a hi-res page of its own until the loader starts.

use mystery_house::{Boot, BootStep, GameData};

use crate::render::{H, W};
use crate::text::TextPage;

/// Where the player is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    Running,
    /// GET is waiting for a key, the cursor at (row, col).
    Key(usize, usize),
    /// The loader starts now.
    Loader,
}

pub struct BootPlayer {
    data: &'static GameData,
    steps: &'static [BootStep],
    /// The first step not yet wholly done.
    next: usize,
    /// How much of the text step at `next` is written.
    written: usize,
    /// The screen shows the text page (else hi-res page 1).
    pub text: bool,
    /// Hi-res page 1: 280 x 192 dots and 40 x 192 colour bits.
    pub pixels: Vec<u8>,
    pub colour_bits: Vec<u8>,
    rows_loaded: usize,
    state: State,
}

/// The row of hi-res page 1 at each 40-byte stretch of memory, in address order.
fn rows_in_memory_order() -> Vec<usize> {
    let mut rows: Vec<usize> = (0..H).collect();
    rows.sort_by_key(|&y| (y & 7) * 0x400 + ((y >> 3) & 7) * 0x80 + (y >> 6) * 0x28);
    rows
}

impl BootPlayer {
    pub fn new(data: &'static GameData) -> BootPlayer {
        BootPlayer {
            data,
            steps: &data.boot,
            next: 0,
            written: 0,
            text: true,
            pixels: vec![0; W * H],
            colour_bits: vec![0; 40 * H],
            rows_loaded: 0,
            state: State::Running,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    /// Play every step due `ms` after the start (or after the last key) onto `page`.
    pub fn advance(&mut self, ms: f64, page: &mut TextPage) -> State {
        while self.state == State::Running {
            let Some(step) = self.steps.get(self.next) else {
                self.state = State::Loader;
                break;
            };
            if step.at > ms {
                break;
            }
            match &step.what {
                Boot::Display { text } => self.text = *text,
                Boot::Hgr => {
                    self.pixels.fill(0);
                    self.colour_bits.fill(0);
                }
                Boot::Text {
                    row,
                    col,
                    text,
                    char_ms,
                } => {
                    let due = if *char_ms > 0.0 {
                        (((ms - step.at) / char_ms) as usize)
                            .saturating_add(1)
                            .min(text.len())
                    } else {
                        text.len()
                    };
                    for (i, c) in text.chars().enumerate().take(due).skip(self.written) {
                        page.put_at(*row, col + i, c);
                    }
                    self.written = due;
                    if due < text.len() {
                        break;
                    }
                }
                Boot::Title { ms: over } => {
                    let order = rows_in_memory_order();
                    let frac = if *over > 0.0 {
                        ((ms - step.at) / over).min(1.0)
                    } else {
                        1.0
                    };
                    let due = (frac * H as f64) as usize;
                    let t = &self.data.title_1987;
                    for &y in &order[self.rows_loaded..due] {
                        self.pixels[y * W..(y + 1) * W]
                            .copy_from_slice(&t.pixels[y * W..(y + 1) * W]);
                        self.colour_bits[y * 40..(y + 1) * 40]
                            .copy_from_slice(&t.colour_bits[y * 40..(y + 1) * 40]);
                    }
                    self.rows_loaded = due;
                    if due < H {
                        break;
                    }
                }
                Boot::Key { row, col } => {
                    page.set_cursor(*row, *col);
                    self.state = State::Key(*row, *col);
                }
                Boot::Loader => self.state = State::Loader,
            }
            self.next += 1;
            self.written = 0;
            self.rows_loaded = 0;
        }
        self.state
    }

    /// Everything up to the key GET waits for, at once.
    pub fn skip_to_key(&mut self, page: &mut TextPage) -> State {
        self.advance(f64::INFINITY, page)
    }

    /// The key GET waited for: the timeline goes on, its times counted from now.
    pub fn key_pressed(&mut self) {
        if let State::Key(..) = self.state {
            self.state = State::Running;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> &'static GameData {
        let json = include_str!("../../../decomp/gamedata.json");
        Box::leak(Box::new(GameData::from_json(json).unwrap()))
    }

    fn row(p: &TextPage, r: usize) -> String {
        (0..40)
            .map(|c| p.char_at(r, c))
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    #[test]
    fn hello_slides_its_credit_in() {
        let mut page = TextPage::default();
        let mut b = BootPlayer::new(data());
        b.advance(20.0, &mut page);
        assert_eq!(row(&page, 6), "       GE");
        b.advance(6800.0, &mut page);
        assert_eq!(row(&page, 6), "       THE PUBLIC DOMAIN EXCHANGE");
        assert_eq!(row(&page, 10), "            (408) 496-0624");
        assert!(b.text);
    }

    #[test]
    fn the_title_loads_then_the_notice_waits_for_a_key() {
        let d = data();
        let mut page = TextPage::default();
        let mut b = BootPlayer::new(d);
        b.advance(13_000.0, &mut page);
        assert!(!b.text);
        let lit = b.pixels.iter().filter(|&&p| p != 0).count();
        assert!(lit > 0 && lit < d.title_1987.pixels.iter().filter(|&&p| p != 0).count());
        assert_eq!(b.skip_to_key(&mut page), State::Key(21, 0));
        assert_eq!(b.pixels, d.title_1987.pixels);
        assert!(b.text);
        assert_eq!(row(&page, 20), "PLAYED IT.");
        assert_eq!(page.cursor(), (21, 0));
        b.key_pressed();
        assert_eq!(b.advance(0.0, &mut page), State::Loader);
    }
}
