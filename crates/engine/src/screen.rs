//! The hi-res screen and the game's drawing routines (decomp/PORTING.md, "The screen").

pub const WIDTH: usize = 280;
pub const HEIGHT: usize = 192;
/// The picture area above the four lines of text.
pub const PICTURE_HEIGHT: usize = 160;

const MAX_Y: u8 = 0xA0;

/// A 280 x 192 one-bit screen with the pen of the relocated Applesoft routines.
///
/// As in Applesoft the pen has a logical position (where HGLIN measures a line from) and a
/// screen position (where the next point is plotted). They differ only after DRAW, which
/// moves the screen position alone.
#[derive(Clone)]
pub struct Screen {
    pixels: Vec<u8>,
    /// Each hi-res byte's colour-group bit, 40 per row. The game draws in white with the bit
    /// clear; only a title picture sets it.
    colour_bits: Vec<u8>,
    x: i32,
    y: i32,
    sx: i32,
    sy: i32,
}

impl Default for Screen {
    fn default() -> Self {
        Screen {
            pixels: vec![0; WIDTH * HEIGHT],
            colour_bits: vec![0; WIDTH / 7 * HEIGHT],
            x: 0,
            y: 0,
            sx: 0,
            sy: 0,
        }
    }
}

impl Screen {
    /// Every pixel, one byte each (1 lit, 0 dark), row by row from the top.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// The colour-group bit of each 7-pixel group, 40 per row (for NTSC colour).
    pub fn colour_bits(&self) -> &[u8] {
        &self.colour_bits
    }

    /// The 280 x 160 picture area, as the pixel hashes take it.
    pub fn picture_area(&self) -> &[u8] {
        &self.pixels[..WIDTH * PICTURE_HEIGHT]
    }

    /// HGR: clear the page. The pen stays where it was.
    pub fn clear(&mut self) {
        self.pixels.fill(0);
        self.colour_bits.fill(0);
    }

    /// Put a whole page on the screen (a title picture loaded over it).
    pub fn load_page(&mut self, pixels: &[u8], colour_bits: &[u8]) {
        self.pixels.copy_from_slice(pixels);
        self.colour_bits.copy_from_slice(colour_bits);
    }

    /// A pixel off the screen is dropped (the game never draws there).
    fn set(&mut self, x: i32, y: i32) {
        if (0..WIDTH as i32).contains(&x) && (0..HEIGHT as i32).contains(&y) {
            self.pixels[y as usize * WIDTH + x as usize] = 1;
        }
    }

    /// HPLOT x,y: both pens move there.
    fn plot(&mut self, x: i32, y: i32) {
        (self.x, self.y, self.sx, self.sy) = (x, y, x, y);
        self.set(x, y);
    }

    /// HPLOT TO x1,y1 (HGLIN): a 4-connected staircase of |dx| + |dy| steps from the screen
    /// pen, with dx and dy measured from the logical pen; the start is plotted again.
    fn line(&mut self, x1: i32, y1: i32) {
        let (dx, dy) = ((x1 - self.x).abs(), (y1 - self.y).abs());
        let step_x = if x1 < self.x { -1 } else { 1 };
        let step_y = if y1 > self.y { 1 } else { -1 };
        let (mut x, mut y) = (self.sx, self.sy);
        let mut err = dx - dy;
        self.set(x, y);
        for _ in 0..dx + dy {
            if err >= 0 {
                x += step_x;
                err -= dy;
            } else {
                y += step_y;
                err += dx;
            }
            self.set(x, y);
        }
        (self.x, self.y, self.sx, self.sy) = (x1, y1, x, y);
    }

    /// `draw_picture`: points offset by (dx, dy) in 8 bits, then clamped to 255 x 160. Each
    /// stroke starts by setting `pen_up`, which carries over between pictures; a picture that
    /// `continues` starts with a line from wherever the pen is.
    pub fn draw_picture(
        &mut self,
        strokes: &[Vec<(u8, u8)>],
        continues: bool,
        dx: u8,
        dy: u8,
        pen_up: &mut bool,
    ) {
        for (i, stroke) in strokes.iter().enumerate() {
            if i > 0 || !continues {
                *pen_up = true;
            }
            for &(x, y) in stroke {
                let x = x.wrapping_add(dx) as i32; // MAX_X is 255: nothing to clamp
                let y = y.wrapping_add(dy).min(MAX_Y) as i32;
                if *pen_up {
                    self.plot(x, y);
                    *pen_up = false;
                } else {
                    self.line(x, y);
                }
            }
        }
    }

    /// Applesoft DRAW at (x, y), scale 1: for each vector, plot if asked, then move one pixel
    /// (0 up, 1 right, 2 down, 3 left). Only the screen pen moves.
    pub fn draw_shape(&mut self, vectors: &[(bool, u8)], x: u8, y: u8) {
        let (x, y) = (x as i32, y as i32);
        (self.x, self.y, self.sx, self.sy) = (x, y, x, y);
        for &(plot, dir) in vectors {
            if plot {
                self.set(self.sx, self.sy);
            }
            match dir & 3 {
                0 => self.sy -= 1,
                1 => self.sx += 1,
                2 => self.sy += 1,
                _ => self.sx -= 1,
            }
        }
    }
}
