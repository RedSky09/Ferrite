use ferrite_core::camera::Camera2D;

/// A textured rectangle to draw this frame.
#[derive(Clone, Copy, Debug)]
pub struct Sprite {
    /// World position of the sprite's center.
    pub position: [f32; 2],
    /// Width and height in world units.
    pub size: [f32; 2],
    /// Tint multiplied with the texture; white leaves it unchanged.
    pub color: [f32; 3],
}

impl Sprite {
    pub fn new(position: [f32; 2], size: [f32; 2]) -> Self {
        Self {
            position,
            size,
            color: [1.0, 1.0, 1.0],
        }
    }

    pub fn with_color(mut self, color: [f32; 3]) -> Self {
        self.color = color;
        self
    }
}

/// The game's handle to the renderer. During `Game::draw` it only collects
/// what should be drawn; the renderer draws it all afterwards in one batch.
pub struct Graphics {
    pub camera: Camera2D,
    /// Background color (red, green, blue), each from 0.0 to 1.0.
    pub clear_color: [f32; 3],
    sprites: Vec<Sprite>,
}

impl Graphics {
    pub fn new() -> Self {
        Self {
            camera: Camera2D::default(),
            clear_color: [0.10, 0.20, 0.30],
            sprites: Vec::new(),
        }
    }

    /// Queues a sprite for this frame. Sprites drawn later appear on top.
    pub fn draw_sprite(&mut self, sprite: Sprite) {
        self.sprites.push(sprite);
    }

    /// Called by the engine before each `Game::draw`. Forgets last frame's sprites.
    pub fn begin_frame(&mut self) {
        self.sprites.clear();
    }

    pub(crate) fn sprites(&self) -> &[Sprite] {
        &self.sprites
    }
}

impl Default for Graphics {
    fn default() -> Self {
        Self::new()
    }
}
