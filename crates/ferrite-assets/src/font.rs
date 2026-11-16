//! A simple built-in bitmap font.
//!
//! Rather than shipping a PNG, the font is described as compact 5x7 bitmap
//! glyphs and baked into an [`Image`] at run time. Each glyph is 5 columns
//! wide and 7 rows tall; bit `r` of a column byte is row `r`, counted from the
//! top. That keeps the engine self-contained (no font file to find, nothing to
//! download) while still going through the same texture path as any other
//! sprite.

use std::collections::HashMap;

use crate::image::Image;

/// Width of one glyph, in pixels.
pub const GLYPH_WIDTH: u32 = 5;
/// Height of one glyph, in pixels.
pub const GLYPH_HEIGHT: u32 = 7;
/// Blank columns inserted between glyphs in the baked atlas.
pub const GLYPH_SPACING: u32 = 1;

/// The printable ASCII range the font covers: `' '` (0x20) through `'~'` (0x7E).
const FIRST_CHAR: u8 = 0x20;
const LAST_CHAR: u8 = 0x7E;

/// One glyph: five column bytes, top row in bit 0.
type Glyph = [u8; GLYPH_WIDTH as usize];

/// A baked bitmap font: a glyph atlas image and where each glyph sits in it.
#[derive(Clone, Debug)]
pub struct Font {
    atlas: Image,
    glyphs: HashMap<u8, crate::spritesheet::Rect>,
}

impl Font {
    /// Bakes the built-in font. The atlas is a single row of glyphs.
    pub fn builtin() -> Self {
        let count = (LAST_CHAR - FIRST_CHAR + 1) as u32;
        let cell_w = GLYPH_WIDTH + GLYPH_SPACING;
        let width = count * cell_w;
        let height = GLYPH_HEIGHT;
        let mut atlas = Image::transparent(width, height);
        let mut glyphs = HashMap::new();

        for (i, code) in (FIRST_CHAR..=LAST_CHAR).enumerate() {
            let x0 = i as u32 * cell_w;
            let glyph = glyph_for(code);
            for (col, bits) in glyph.iter().enumerate() {
                for row in 0..GLYPH_HEIGHT {
                    if bits & (1 << row) != 0 {
                        atlas.set_pixel(x0 + col as u32, row, [255, 255, 255, 255]);
                    }
                }
            }
            glyphs.insert(
                code,
                crate::spritesheet::Rect::new(x0, 0, GLYPH_WIDTH, GLYPH_HEIGHT),
            );
        }

        Self { atlas, glyphs }
    }

    /// The glyph atlas as an image, ready to upload as a texture.
    pub fn atlas(&self) -> &Image {
        &self.atlas
    }

    /// The region of `c` in the atlas, or `None` for unsupported characters.
    pub fn glyph(&self, c: char) -> Option<crate::spritesheet::Rect> {
        let code = c as u32;
        if code > u8::MAX as u32 {
            return None;
        }
        self.glyphs.get(&(code as u8)).copied()
    }

    /// Advances, in pixels, from one glyph's origin to the next.
    pub fn advance(&self) -> u32 {
        GLYPH_WIDTH + GLYPH_SPACING
    }

    /// Lays out `text` into positioned glyphs, starting at `(x, y)` (top-left).
    ///
    /// Unknown characters are skipped but still advance the cursor, so text
    /// does not collapse. `\n` moves to the next line.
    pub fn layout(&self, text: &str, mut x: i32, mut y: i32) -> Vec<(crate::spritesheet::Rect, i32, i32)> {
        let mut out = Vec::new();
        let line_height = GLYPH_HEIGHT as i32 + 1;
        for c in text.chars() {
            if c == '\n' {
                x = 0;
                y += line_height;
                continue;
            }
            if let Some(rect) = self.glyph(c) {
                out.push((rect, x, y));
            }
            x += self.advance() as i32;
        }
        out
    }

    /// The pixel width of `text` if drawn on one line.
    pub fn measure_width(&self, text: &str) -> u32 {
        text.chars().count() as u32 * self.advance()
    }
}

/// The dot pattern for one character, or a blank glyph if it is unsupported.
fn glyph_for(c: u8) -> Glyph {
    match c {
        b' ' => [0x00, 0x00, 0x00, 0x00, 0x00],
        b'A' | b'a' => [0x7E, 0x11, 0x11, 0x11, 0x7E],
        b'B' | b'b' => [0x7F, 0x49, 0x49, 0x49, 0x36],
        b'C' | b'c' => [0x3E, 0x41, 0x41, 0x41, 0x22],
        b'D' | b'd' => [0x7F, 0x41, 0x41, 0x22, 0x1C],
        b'E' | b'e' => [0x7F, 0x49, 0x49, 0x49, 0x41],
        b'F' | b'f' => [0x7F, 0x09, 0x09, 0x09, 0x01],
        b'G' | b'g' => [0x3E, 0x41, 0x49, 0x49, 0x7A],
        b'H' | b'h' => [0x7F, 0x08, 0x08, 0x08, 0x7F],
        b'I' | b'i' => [0x00, 0x41, 0x7F, 0x41, 0x00],
        b'J' | b'j' => [0x20, 0x40, 0x41, 0x3F, 0x01],
        b'K' | b'k' => [0x7F, 0x08, 0x14, 0x22, 0x41],
        b'L' | b'l' => [0x7F, 0x40, 0x40, 0x40, 0x40],
        b'M' | b'm' => [0x7F, 0x02, 0x0C, 0x02, 0x7F],
        b'N' | b'n' => [0x7F, 0x04, 0x08, 0x10, 0x7F],
        b'O' | b'o' => [0x3E, 0x41, 0x41, 0x41, 0x3E],
        b'P' | b'p' => [0x7F, 0x09, 0x09, 0x09, 0x06],
        b'Q' | b'q' => [0x3E, 0x41, 0x51, 0x21, 0x5E],
        b'R' | b'r' => [0x7F, 0x09, 0x19, 0x29, 0x46],
        b'S' | b's' => [0x46, 0x49, 0x49, 0x49, 0x31],
        b'T' | b't' => [0x01, 0x01, 0x7F, 0x01, 0x01],
        b'U' | b'u' => [0x3F, 0x40, 0x40, 0x40, 0x3F],
        b'V' | b'v' => [0x1F, 0x20, 0x40, 0x20, 0x1F],
        b'W' | b'w' => [0x3F, 0x40, 0x38, 0x40, 0x3F],
        b'X' | b'x' => [0x63, 0x14, 0x08, 0x14, 0x63],
        b'Y' | b'y' => [0x07, 0x08, 0x70, 0x08, 0x07],
        b'Z' | b'z' => [0x61, 0x51, 0x49, 0x45, 0x43],
        b'0' => [0x3E, 0x51, 0x49, 0x45, 0x3E],
        b'1' => [0x00, 0x42, 0x7F, 0x40, 0x00],
        b'2' => [0x42, 0x61, 0x51, 0x49, 0x46],
        b'3' => [0x21, 0x41, 0x45, 0x4B, 0x31],
        b'4' => [0x18, 0x14, 0x12, 0x7F, 0x10],
        b'5' => [0x27, 0x45, 0x45, 0x45, 0x39],
        b'6' => [0x3C, 0x4A, 0x49, 0x49, 0x30],
        b'7' => [0x01, 0x71, 0x09, 0x05, 0x03],
        b'8' => [0x36, 0x49, 0x49, 0x49, 0x36],
        b'9' => [0x06, 0x49, 0x49, 0x29, 0x1E],
        b'.' => [0x00, 0x60, 0x60, 0x00, 0x00],
        b',' => [0x00, 0x50, 0x30, 0x00, 0x00],
        b'!' => [0x00, 0x00, 0x5F, 0x00, 0x00],
        b'?' => [0x02, 0x01, 0x51, 0x09, 0x06],
        b':' => [0x00, 0x36, 0x36, 0x00, 0x00],
        b';' => [0x00, 0x56, 0x36, 0x00, 0x00],
        b'-' => [0x08, 0x08, 0x08, 0x08, 0x08],
        b'_' => [0x40, 0x40, 0x40, 0x40, 0x40],
        b'+' => [0x08, 0x08, 0x3E, 0x08, 0x08],
        b'=' => [0x14, 0x14, 0x14, 0x14, 0x14],
        b'/' => [0x20, 0x10, 0x08, 0x04, 0x02],
        b'\\' => [0x02, 0x04, 0x08, 0x10, 0x20],
        b'(' => [0x00, 0x1C, 0x22, 0x41, 0x00],
        b')' => [0x00, 0x41, 0x22, 0x1C, 0x00],
        b'[' => [0x00, 0x7F, 0x41, 0x41, 0x00],
        b']' => [0x00, 0x41, 0x41, 0x7F, 0x00],
        b'<' => [0x08, 0x14, 0x22, 0x41, 0x00],
        b'>' => [0x00, 0x41, 0x22, 0x14, 0x08],
        b'\'' => [0x00, 0x05, 0x03, 0x00, 0x00],
        b'"' => [0x00, 0x07, 0x00, 0x07, 0x00],
        b'*' => [0x14, 0x08, 0x3E, 0x08, 0x14],
        b'#' => [0x14, 0x7F, 0x14, 0x7F, 0x14],
        b'%' => [0x23, 0x13, 0x08, 0x64, 0x62],
        b'@' => [0x3E, 0x41, 0x5D, 0x55, 0x3E],
        b'&' => [0x36, 0x49, 0x55, 0x22, 0x50],
        b'$' => [0x24, 0x2A, 0x7F, 0x2A, 0x12],
        // Anything else in range renders as blank but still advances.
        _ => [0x00, 0x00, 0x00, 0x00, 0x00],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_printable_char_has_a_glyph() {
        let font = Font::builtin();
        for code in FIRST_CHAR..=LAST_CHAR {
            let c = code as char;
            assert!(font.glyph(c).is_some(), "missing glyph for {c:?}");
        }
    }

    #[test]
    fn atlas_has_room_for_every_glyph() {
        let font = Font::builtin();
        let count = (LAST_CHAR - FIRST_CHAR + 1) as u32;
        assert_eq!(font.atlas().width(), count * (GLYPH_WIDTH + GLYPH_SPACING));
        assert_eq!(font.atlas().height(), GLYPH_HEIGHT);
    }

    #[test]
    fn space_is_blank_but_a_letter_is_not() {
        let font = Font::builtin();
        let space = font.glyph(' ').unwrap();
        let a = font.glyph('A').unwrap();
        // The space glyph region is entirely transparent.
        let mut space_pixels = 0;
        for y in space.y..space.y + space.height {
            for x in space.x..space.x + space.width {
                if font.atlas().pixel(x, y)[3] != 0 {
                    space_pixels += 1;
                }
            }
        }
        assert_eq!(space_pixels, 0, "space should be empty");
        // 'A' has at least one lit pixel.
        let mut a_pixels = 0;
        for y in a.y..a.y + a.height {
            for x in a.x..a.x + a.width {
                if font.atlas().pixel(x, y)[3] != 0 {
                    a_pixels += 1;
                }
            }
        }
        assert!(a_pixels > 0, "'A' should have lit pixels");
    }

    #[test]
    fn layout_advances_per_character() {
        let font = Font::builtin();
        let glyphs = font.layout("AB", 10, 5);
        assert_eq!(glyphs.len(), 2);
        assert_eq!(glyphs[0].1, 10); // first starts at x
        assert_eq!(glyphs[1].1, 10 + font.advance() as i32);
        assert_eq!(glyphs[0].2, 5); // same baseline
    }

    #[test]
    fn layout_wraps_on_newline() {
        let font = Font::builtin();
        let glyphs = font.layout("A\nB", 7, 3);
        assert_eq!(glyphs[1].1, 0); // reset x
        assert!(glyphs[1].2 > 3); // lower line
    }

    #[test]
    fn measure_width_counts_advances() {
        let font = Font::builtin();
        assert_eq!(font.measure_width(""), 0);
        assert_eq!(font.measure_width("ABC"), 3 * font.advance());
    }

    #[test]
    fn unsupported_characters_are_skipped_but_advance() {
        let font = Font::builtin();
        // A tab is not in the atlas; it should not appear but still moves x.
        let glyphs = font.layout("a\tb", 0, 0);
        assert_eq!(glyphs.len(), 2);
        assert_eq!(glyphs[1].1, 2 * font.advance() as i32);
    }
}
