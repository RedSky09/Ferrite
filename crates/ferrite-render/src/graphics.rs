use ferrite_core::camera::Camera2D;

/// Which texture a queued sprite should sample.
///
/// Games never see GPU objects. They hold an `Atlas` handle that the engine
/// minted when the texture was uploaded, and queue sprites against it. Two
/// sprites with different atlases cannot share a draw call, so the renderer
/// groups by this value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Atlas(pub(crate) u32);

/// The uv rectangle of a sprite within its atlas: `[u0, v0, u1, v1]`.
///
/// v grows downward, so `[0, 0, 1, 1]` is the whole texture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UvRect {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

impl UvRect {
    /// The whole texture.
    pub const FULL: Self = Self {
        u0: 0.0,
        v0: 0.0,
        u1: 1.0,
        v1: 1.0,
    };

    pub const fn new(u0: f32, v0: f32, u1: f32, v1: f32) -> Self {
        Self { u0, v0, u1, v1 }
    }
}

impl Default for UvRect {
    fn default() -> Self {
        Self::FULL
    }
}

/// A textured rectangle to draw this frame.
#[derive(Clone, Copy, Debug)]
pub struct Sprite {    /// World position of the sprite's center.
    pub position: [f32; 2],
    /// Width and height in world units.
    pub size: [f32; 2],
    /// Tint multiplied with the texture; white leaves it unchanged.
    pub color: [f32; 3],
    /// Which atlas to sample.
    pub atlas: Atlas,
    /// Which part of the atlas to sample.
    pub uv: UvRect,
}

impl Sprite {
    pub fn new(position: [f32; 2], size: [f32; 2]) -> Self {
        Self {
            position,
            size,
            color: [1.0, 1.0, 1.0],
            atlas: Atlas::WHITE,
            uv: UvRect::FULL,
        }
    }

    pub fn with_color(mut self, color: [f32; 3]) -> Self {
        self.color = color;
        self
    }

    pub fn with_atlas(mut self, atlas: Atlas) -> Self {
        self.atlas = atlas;
        self
    }

    pub fn with_uv(mut self, uv: UvRect) -> Self {
        self.uv = uv;
        self
    }
}

impl Atlas {
    /// A one-pixel opaque white texture, used when a sprite has no atlas.
    ///
    /// Tinting it any color produces a solid rectangle, which is what the
    /// engine's untextured sprites have always been.
    pub const WHITE: Atlas = Atlas(0);
}

/// A horizontal alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// A vertical alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VAlign {
    Top,
    Middle,
    Bottom,
}

/// Where a block of text sits relative to a point.
///
/// The pair says which corner/edge of the text's bounding box is placed at the
/// caller's position. `Anchor::CENTER` centers the text on the point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anchor {
    pub h: Align,
    pub v: VAlign,
}

impl Anchor {
    pub const TOP_LEFT: Self = Self {
        h: Align::Left,
        v: VAlign::Top,
    };
    pub const TOP_CENTER: Self = Self {
        h: Align::Center,
        v: VAlign::Top,
    };
    pub const CENTER: Self = Self {
        h: Align::Center,
        v: VAlign::Middle,
    };
    pub const BOTTOM_CENTER: Self = Self {
        h: Align::Center,
        v: VAlign::Bottom,
    };

    pub const fn new(h: Align, v: VAlign) -> Self {
        Self { h, v }
    }
}

impl Default for Anchor {
    fn default() -> Self {
        Self::TOP_LEFT
    }
}

/// The game's handle to the renderer. During `Game::draw` it only collects
/// what should be drawn; the renderer draws it all afterwards.
pub struct Graphics {
    pub camera: Camera2D,
    /// Background color (red, green, blue), each from 0.0 to 1.0.
    pub clear_color: [f32; 3],
    sprites: Vec<Sprite>,
    /// Newly uploaded textures, waiting for the renderer to move them to the GPU.
    pub(crate) uploads: Vec<TextureUpload>,
    next_atlas: u32,
}

impl Graphics {
    pub fn new() -> Self {
        Self {
            camera: Camera2D::default(),
            clear_color: [0.10, 0.20, 0.30],
            sprites: Vec::new(),
            uploads: Vec::new(),
            // 0 is reserved for the built-in white texture.
            next_atlas: 1,
        }
    }

    /// Uploads RGBA8 pixels and returns a handle to use in sprites.
    ///
    /// The upload is deferred: the engine performs it before the next draw,
    /// so this is safe to call during `update` as well as `draw`.
    ///
    /// # Panics
    /// Panics if `rgba.len() != width * height * 4`.
    pub fn upload_texture(&mut self, rgba: &[u8], width: u32, height: u32) -> Atlas {
        assert_eq!(
            rgba.len(),
            (width * height * 4) as usize,
            "pixel data does not match the texture size"
        );
        let atlas = Atlas(self.next_atlas);
        self.next_atlas += 1;
        self.uploads.push(TextureUpload {
            atlas,
            rgba: rgba.to_vec(),
            width,
            height,
        });
        atlas
    }

    /// Queues a sprite for this frame. Sprites drawn later appear on top.
    pub fn draw_sprite(&mut self, sprite: Sprite) {
        self.sprites.push(sprite);
    }

    /// Uploads an [`Image`] and returns its atlas handle.
    ///
    /// Convenience wrapper over [`Graphics::upload_texture`].
    pub fn upload_image(&mut self, image: &ferrite_assets::Image) -> Atlas {
        self.upload_texture(image.rgba(), image.width(), image.height())
    }

    /// Draws a whole spritesheet frame from `atlas`, sized in world units.
    ///
    /// `image_width` and `image_height` are the pixel size of the texture that
    /// `atlas` was uploaded from, needed to turn the frame's pixel rect into uv.
    pub fn draw_frame(
        &mut self,
        atlas: Atlas,
        rect: ferrite_assets::Rect,
        image_width: u32,
        image_height: u32,
        position: [f32; 2],
        size: [f32; 2],
    ) {
        let [u0, v0, u1, v1] = rect.uv(image_width, image_height);
        self.draw_sprite(
            Sprite::new(position, size)
                .with_atlas(atlas)
                .with_uv(UvRect::new(u0, v0, u1, v1)),
        );
    }

    /// The world-space size of `text` drawn at `pixel_size`.
    ///
    /// Returns `(width, height)`. The width is the widest line; the height
    /// covers the glyphs: each line is one glyph tall, plus a one-pixel gap
    /// between lines. The box hugs the glyphs, so centering single-line text
    /// centers the letters rather than including a trailing blank row.
    pub fn measure_text(
        font: &ferrite_assets::Font,
        text: &str,
        pixel_size: f32,
    ) -> [f32; 2] {
        let glyph_h = ferrite_assets::font::GLYPH_HEIGHT as f32 * pixel_size;
        let gap = pixel_size;
        let advance = font.advance() as f32 * pixel_size;
        let mut widest = 0f32;
        for line in text.split('\n') {
            widest = widest.max(line.chars().count() as f32 * advance);
        }
        let lines = text.split('\n').count() as f32;
        // `lines` glyphs high, with a gap between each pair of lines.
        let height = lines * glyph_h + (lines - 1.0).max(0.0) * gap;
        [widest, height]
    }

    /// Draws `text` with its top-left corner at `origin`, in white.
    ///
    /// `pixel_size` is how many world units one font pixel covers, and the
    /// atlas must have been uploaded from `font.atlas()`.
    pub fn draw_text(
        &mut self,
        font: &ferrite_assets::Font,
        atlas: Atlas,
        text: &str,
        origin: [f32; 2],
        pixel_size: f32,
    ) {
        self.draw_text_anchored(font, atlas, text, origin, pixel_size, Anchor::TOP_LEFT);
    }

    /// Draws `text` aligned to `position` using `anchor`, in white.
    ///
    /// With [`Anchor::CENTER`] the text is centered on `position`, which is the
    /// usual way to put a title or a "GAME OVER" banner in the middle of the
    /// screen. Multi-line text (`\n`) is handled.
    pub fn draw_text_anchored(
        &mut self,
        font: &ferrite_assets::Font,
        atlas: Atlas,
        text: &str,
        position: [f32; 2],
        pixel_size: f32,
        anchor: Anchor,
    ) {
        self.draw_text_colored(font, atlas, text, position, pixel_size, anchor, [1.0, 1.0, 1.0]);
    }

    /// The full-control text call: alignment and tint.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_text_colored(
        &mut self,
        font: &ferrite_assets::Font,
        atlas: Atlas,
        text: &str,
        position: [f32; 2],
        pixel_size: f32,
        anchor: Anchor,
        color: [f32; 3],
    ) {
        let aw = font.atlas().width();
        let ah = font.atlas().height();
        let [block_w, block_h] = Self::measure_text(font, text, pixel_size);

        // Offset from the anchor point to the text box's top-left corner.
        // Font/world y is up here: the text box extends *downward* from its
        // top-left, so anchoring to the middle or bottom moves the top edge up.
        let offset_x = match anchor.h {
            Align::Left => 0.0,
            Align::Center => -block_w * 0.5,
            Align::Right => -block_w,
        };
        let offset_y = match anchor.v {
            VAlign::Top => 0.0,
            VAlign::Middle => block_h * 0.5,
            VAlign::Bottom => block_h,
        };
        let top_left = [position[0] + offset_x, position[1] + offset_y];

        for (rect, x, y) in font.layout(text, 0, 0) {
            let [u0, v0, u1, v1] = rect.uv(aw, ah);
            // Font layout is top-left based; sprites are centered.
            let w = rect.width as f32 * pixel_size;
            let h = rect.height as f32 * pixel_size;
            let cx = top_left[0] + x as f32 * pixel_size + w * 0.5;
            let cy = top_left[1] - (y as f32 * pixel_size) - h * 0.5;
            self.draw_sprite(
                Sprite::new([cx, cy], [w, h])
                    .with_atlas(atlas)
                    .with_uv(UvRect::new(u0, v0, u1, v1))
                    .with_color(color),
            );
        }
    }

    /// Called by the engine before each `Game::draw`. Forgets last frame's sprites.
    pub fn begin_frame(&mut self) {
        self.sprites.clear();
    }

    pub(crate) fn sprites(&self) -> &[Sprite] {
        &self.sprites
    }

    pub(crate) fn take_uploads(&mut self) -> Vec<TextureUpload> {
        std::mem::take(&mut self.uploads)
    }
}

impl Default for Graphics {
    fn default() -> Self {
        Self::new()
    }
}

/// A texture waiting to be uploaded. Owned by `Graphics` until the renderer
/// picks it up.
pub(crate) struct TextureUpload {
    pub(crate) atlas: Atlas,
    pub(crate) rgba: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uploads_get_distinct_handles() {
        let mut gfx = Graphics::new();
        let a = gfx.upload_texture(&[255; 4], 1, 1);
        let b = gfx.upload_texture(&[255; 4], 1, 1);
        assert_ne!(a, b);
        // The reserved white handle is never handed out.
        assert_ne!(a, Atlas::WHITE);
        assert_ne!(b, Atlas::WHITE);
    }

    #[test]
    fn begin_frame_clears_sprites_but_keeps_uploads_until_taken() {
        let mut gfx = Graphics::new();
        gfx.upload_texture(&[255; 4], 1, 1);
        gfx.draw_sprite(Sprite::new([0.0, 0.0], [1.0, 1.0]));
        gfx.begin_frame();
        assert!(gfx.sprites().is_empty());
        assert_eq!(gfx.take_uploads().len(), 1);
        // Taken means gone.
        assert!(gfx.take_uploads().is_empty());
    }

    #[test]
    fn default_sprite_uses_the_whole_white_atlas() {
        let s = Sprite::new([0.0, 0.0], [1.0, 1.0]);
        assert_eq!(s.atlas, Atlas::WHITE);
        assert_eq!(s.uv, UvRect::FULL);
    }

    #[test]
    fn draw_frame_turns_a_pixel_rect_into_uv() {
        let mut gfx = Graphics::new();
        // A 2x1 "atlas" of 8x8 pixels; frame at (8, 0) is the right half.
        let rect = ferrite_assets::Rect::new(8, 0, 8, 8);
        gfx.draw_frame(Atlas(5), rect, 16, 8, [0.0, 0.0], [1.0, 1.0]);
        let s = &gfx.sprites()[0];
        assert_eq!(s.atlas, Atlas(5));
        // u from 0.5 to 1.0, v from 0.0 to 1.0.
        assert!((s.uv.u0 - 0.5).abs() < 1e-6);
        assert!((s.uv.u1 - 1.0).abs() < 1e-6);
        assert!((s.uv.v0 - 0.0).abs() < 1e-6);
        assert!((s.uv.v1 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn draw_text_emits_one_sprite_per_glyph_positioned_left_to_right() {
        let mut gfx = Graphics::new();
        let font = ferrite_assets::Font::builtin();
        gfx.draw_text(&font, Atlas(2), "AB", [0.0, 0.0], 1.0);
        let sprites = gfx.sprites();
        assert_eq!(sprites.len(), 2);
        // Second glyph is one advance to the right of the first.
        let advance = (ferrite_assets::font::GLYPH_WIDTH + ferrite_assets::font::GLYPH_SPACING) as f32;
        assert!((sprites[1].position[0] - sprites[0].position[0] - advance).abs() < 1e-6);
        // Both use the font atlas.
        assert_eq!(sprites[0].atlas, Atlas(2));
        assert_eq!(sprites[1].atlas, Atlas(2));
        // The glyph size is one font cell scaled by pixel_size.
        assert!((sprites[0].size[0] - ferrite_assets::font::GLYPH_WIDTH as f32).abs() < 1e-6);
    }

    #[test]
    fn draw_text_places_the_first_glyph_centered_on_the_origin_cell() {
        let mut gfx = Graphics::new();
        let font = ferrite_assets::Font::builtin();
        gfx.draw_text(&font, Atlas(2), "A", [10.0, 5.0], 2.0);
        let s = &gfx.sprites()[0];
        let half_w = ferrite_assets::font::GLYPH_WIDTH as f32; // (width*2)/2
        let half_h = ferrite_assets::font::GLYPH_HEIGHT as f32;
        // Top-left origin + half the scaled size.
        assert!((s.position[0] - (10.0 + half_w)).abs() < 1e-6);
        assert!((s.position[1] - (5.0 - half_h)).abs() < 1e-6);
    }

    #[test]
    fn measure_text_matches_layout_advances() {
        let font = ferrite_assets::Font::builtin();
        let advance = font.advance() as f32;
        // 3 characters on one line.
        let [w, _h] = Graphics::measure_text(&font, "ABC", 1.0);
        assert!((w - 3.0 * advance).abs() < 1e-5);
        // Two lines is taller than one.
        let [_, h1] = Graphics::measure_text(&font, "A", 1.0);
        let [_, h2] = Graphics::measure_text(&font, "A\nB", 1.0);
        assert!(h2 > h1);
    }

    #[test]
    fn center_anchor_symmetrically_shifts_the_block() {
        let mut gfx = Graphics::new();
        let font = ferrite_assets::Font::builtin();
        let text = "AB";

        // Draw at top-left origin 0,0.
        gfx.draw_text(&font, Atlas(1), text, [0.0, 0.0], 1.0);
        let tl: Vec<_> = gfx.sprites().iter().map(|s| s.position).collect();
        gfx.begin_frame();

        // Draw the same text centered on the block's own center, which should
        // reproduce the exact same positions.
        let [w, h] = Graphics::measure_text(&font, text, 1.0);
        let center = [w * 0.5, -h * 0.5];
        gfx.draw_text_anchored(&font, Atlas(1), text, center, 1.0, Anchor::CENTER);
        let centered: Vec<_> = gfx.sprites().iter().map(|s| s.position).collect();

        assert_eq!(tl.len(), centered.len());
        for (a, b) in tl.iter().zip(centered.iter()) {
            assert!((a[0] - b[0]).abs() < 1e-5, "x {a:?} vs {b:?}");
            assert!((a[1] - b[1]).abs() < 1e-5, "y {a:?} vs {b:?}");
        }
    }

    #[test]
    fn right_and_top_anchors_shift_opposite_ways() {
        let font = ferrite_assets::Font::builtin();
        let text = "A";
        let [w, _] = Graphics::measure_text(&font, text, 1.0);

        let mut gfx = Graphics::new();
        gfx.draw_text_anchored(&font, Atlas(1), text, [0.0, 0.0], 1.0, Anchor::TOP_LEFT);
        let left_x = gfx.sprites()[0].position[0];
        gfx.begin_frame();
        gfx.draw_text_anchored(&font, Atlas(1), text, [0.0, 0.0], 1.0, Anchor::TOP_CENTER);
        let center_x = gfx.sprites()[0].position[0];
        gfx.begin_frame();
        gfx.draw_text_anchored(
            &font,
            Atlas(1),
            text,
            [0.0, 0.0],
            1.0,
            Anchor::new(Align::Right, VAlign::Top),
        );
        let right_x = gfx.sprites()[0].position[0];

        // Left is rightmost-shifted by 0, center by half, right by full width.
        assert!(center_x < left_x);
        assert!((left_x - center_x - w * 0.5).abs() < 1e-5);
        assert!(right_x < center_x);
        assert!((left_x - right_x - w).abs() < 1e-5);
    }
}
