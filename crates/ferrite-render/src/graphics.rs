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
pub struct Sprite {
    /// World position of the sprite's center.
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

    /// Draws `text` with a bitmap font atlas.
    ///
    /// `origin` is the top-left of the text in world units. `pixel_size` is how
    /// many world units one font pixel covers, and the atlas must have been
    /// uploaded from `font.atlas()`.
    pub fn draw_text(
        &mut self,
        font: &ferrite_assets::Font,
        atlas: Atlas,
        text: &str,
        origin: [f32; 2],
        pixel_size: f32,
    ) {
        let aw = font.atlas().width();
        let ah = font.atlas().height();
        for (rect, x, y) in font.layout(text, 0, 0) {
            let [u0, v0, u1, v1] = rect.uv(aw, ah);
            // Font layout is top-left based; sprites are centered.
            let w = rect.width as f32 * pixel_size;
            let h = rect.height as f32 * pixel_size;
            let cx = origin[0] + x as f32 * pixel_size + w * 0.5;
            let cy = origin[1] - (y as f32 * pixel_size) - h * 0.5;
            self.draw_sprite(
                Sprite::new([cx, cy], [w, h])
                    .with_atlas(atlas)
                    .with_uv(UvRect::new(u0, v0, u1, v1)),
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
}
