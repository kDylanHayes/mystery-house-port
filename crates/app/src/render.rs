//! The 280 x 192 Apple screen as the player sees it, and its scaling into the window.

use crate::font::{glyph, CELL_H, CELL_W};
use crate::text::{TextPage, COLS, ROWS};

pub const W: usize = 280;
pub const H: usize = 192;

pub type Rgb = [u8; 3];

const BLACK: Rgb = [0, 0, 0];
const WHITE: Rgb = [255, 255, 255];
// The NTSC artefact colours of the decompilation's pictures.hires_color: a lone dot in an
// even column is green (colour group 0) or orange (group 1), in an odd one violet or blue.
const GREEN: Rgb = [20, 245, 60];
const VIOLET: Rgb = [255, 68, 253];
const ORANGE: Rgb = [255, 106, 60];
const BLUE: Rgb = [20, 207, 253];

/// Which page the display shows.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum View {
    /// Hi-res above, the text page's bottom four lines below.
    Mixed,
    /// The whole text page.
    Text,
    /// The whole hi-res page (a title picture).
    Graphics,
}

/// How the picture is coloured.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Colour {
    /// White on black, one dot per pixel.
    White,
    /// The colours an NTSC monitor shows: single dots in green, violet, orange or blue.
    Ntsc,
}

/// How the screen is scaled into the window.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Aspect {
    /// Square pixels, the largest whole multiple that fits.
    Square,
    /// The 4:3 picture of a monitor, as large as fits.
    FourThree,
}

/// One row of the hi-res page in colour. `bits` are the 280 dots, `groups` the 40 colour-group
/// bits. A dot next to another is white; a lone dot takes its column's colour; a gap between
/// two dots is filled with the colour between them.
fn ntsc_row(bits: &[u8], groups: &[u8], out: &mut [Rgb]) {
    let colour = |x: usize, on_even: bool| match (groups[x / 7] != 0, on_even) {
        (false, true) => GREEN,
        (false, false) => VIOLET,
        (true, true) => ORANGE,
        (true, false) => BLUE,
    };
    for x in 0..W {
        let left = x > 0 && bits[x - 1] != 0;
        let right = x + 1 < W && bits[x + 1] != 0;
        out[x] = if bits[x] != 0 {
            if left || right {
                WHITE
            } else {
                colour(x, x % 2 == 0)
            }
        } else if left && right {
            colour(x, (x - 1) % 2 == 0)
        } else {
            BLACK
        };
    }
}

fn hires_rows(
    pixels: &[u8],
    groups: &[u8],
    colour: Colour,
    rows: std::ops::Range<usize>,
    out: &mut [Rgb],
) {
    for y in rows {
        let bits = &pixels[y * W..(y + 1) * W];
        let row = &mut out[y * W..(y + 1) * W];
        match colour {
            Colour::White => {
                for (o, &b) in row.iter_mut().zip(bits) {
                    *o = if b != 0 { WHITE } else { BLACK };
                }
            }
            Colour::Ntsc => ntsc_row(bits, &groups[y * 40..(y + 1) * 40], row),
        }
    }
}

/// Text rows `rows` of the page, drawn from screen line `top`; `cursor` is shown inverse.
fn text_rows(
    page: &TextPage,
    rows: std::ops::Range<usize>,
    top: usize,
    cursor: Option<(usize, usize)>,
    out: &mut [Rgb],
) {
    for (line, r) in rows.enumerate() {
        for c in 0..COLS {
            let cell = glyph(page.char_at(r, c));
            let inverse = cursor == Some((r, c));
            for (dy, bits) in cell.iter().enumerate() {
                let y = top + (line * CELL_H) + dy;
                for dx in 0..CELL_W {
                    let on = (bits >> (CELL_W - 1 - dx)) & 1 != 0;
                    out[y * W + c * CELL_W + dx] = if on != inverse { WHITE } else { BLACK };
                }
            }
        }
    }
}

/// The screen: `pixels` and `groups` are the hi-res page (280 x 192 dots, 40 x 192 colour
/// bits), `page` the text page, `cursor` the flashing cursor when it is lit.
pub fn compose(
    pixels: &[u8],
    groups: &[u8],
    page: &TextPage,
    view: View,
    colour: Colour,
    cursor: Option<(usize, usize)>,
    out: &mut [Rgb],
) {
    match view {
        View::Graphics => hires_rows(pixels, groups, colour, 0..H, out),
        View::Mixed => {
            hires_rows(pixels, groups, colour, 0..160, out);
            text_rows(page, ROWS - 4..ROWS, 160, cursor, out);
        }
        View::Text => text_rows(page, 0..ROWS, 0, cursor, out),
    }
}

/// Where the screen goes in a `dw` x `dh` window: (x, y, width, height).
pub fn placement(dw: usize, dh: usize, aspect: Aspect) -> (usize, usize, usize, usize) {
    let (w, h) = match aspect {
        Aspect::Square => {
            let s = (dw / W).min(dh / H).max(1);
            (W * s, H * s)
        }
        Aspect::FourThree => {
            if dw * 3 > dh * 4 {
                (dh * 4 / 3, dh)
            } else {
                (dw, dw * 3 / 4)
            }
        }
    };
    let (w, h) = (w.min(dw), h.min(dh));
    ((dw - w) / 2, (dh - h) / 2, w, h)
}

/// Scale the screen into an RGBA frame of `dw` x `dh`, black around it.
pub fn blit(screen: &[Rgb], frame: &mut [u8], dw: usize, dh: usize, aspect: Aspect) {
    frame
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .for_each(|p| p.copy_from_slice(&[0, 0, 0, 255]));
    let (x0, y0, w, h) = placement(dw, dh, aspect);
    if w == 0 || h == 0 {
        return;
    }
    let cols: Vec<usize> = (0..w).map(|x| x * W / w).collect();
    for y in 0..h {
        let src = &screen[(y * H / h) * W..][..W];
        let dst = &mut frame[((y0 + y) * dw + x0) * 4..][..w * 4];
        for (d, &sx) in dst.as_chunks_mut::<4>().0.iter_mut().zip(&cols) {
            d[..3].copy_from_slice(&src[sx]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_scaling_is_a_whole_multiple() {
        assert_eq!(placement(1000, 700, Aspect::Square), (80, 62, 840, 576));
        assert_eq!(placement(100, 100, Aspect::Square), (0, 0, 100, 100));
        assert_eq!(placement(1600, 900, Aspect::FourThree), (200, 0, 1200, 900));
    }

    #[test]
    fn lone_dots_are_coloured_and_pairs_white() {
        let mut bits = vec![0u8; W];
        let groups = vec![0u8; 40];
        bits[10] = 1;
        bits[20] = 1;
        bits[21] = 1;
        let mut out = vec![BLACK; W];
        ntsc_row(&bits, &groups, &mut out);
        assert_eq!(
            (out[10], out[20], out[21], out[11]),
            (GREEN, WHITE, WHITE, BLACK)
        );
    }

    #[test]
    fn mixed_view_shows_the_last_four_lines() {
        let mut page = TextPage::default();
        page.print("HELLO");
        let mut out = vec![BLACK; W * H];
        compose(
            &vec![0; W * H],
            &vec![0; 40 * H],
            &page,
            View::Mixed,
            Colour::White,
            None,
            &mut out,
        );
        // The H of HELLO: column 1 of its cell is lit on the first row of the last line.
        assert_eq!(out[(184) * W + 1], WHITE);
        assert_eq!(out[(184) * W], BLACK);
    }
}
