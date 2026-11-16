//! M4 demo: load a PNG, slice it into an animated spritesheet, and draw text
//! with the built-in bitmap font.
//!
//! Controls: Space pauses/resumes the animation, A/D cycles the animation
//! speed. Everything is drawn from two textures: the spritesheet atlas and the
//! font atlas, so the frame is two draw calls.

use ferrite::{
    run, Animation, Atlas, Config, Font, Game, Graphics, Image, Input, Key, Spritesheet,
};

struct Demo {
    // Atlas handles are minted on the first frame, once the renderer exists.
    spinner_atlas: Option<Atlas>,
    font_atlas: Option<Atlas>,
    sheet: Spritesheet,
    animation: Animation,
    font: Font,
    elapsed_ms: f64,
    speed: f32,
    paused: bool,
    player: [f32; 2],
}

impl Demo {
    fn new() -> Self {
        // Asset paths are relative to the crate that owns the example.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../ferrite-assets/examples/assets/spinner.png");
        let image = Image::load_png(&path).expect("failed to load spinner.png");

        // The image is a 4x1 strip of 16x16 frames.
        let sheet = Spritesheet::from_grid(image, 16, 16);
        let frames: Vec<_> = (0..sheet.frame_count())
            .map(|i| sheet.frame_rect_index(i))
            .collect();
        let animation = Animation::uniform(frames, 120, true);

        Self {
            spinner_atlas: None,
            font_atlas: None,
            sheet,
            animation,
            font: Font::builtin(),
            elapsed_ms: 0.0,
            speed: 1.0,
            paused: false,
            player: [0.0, 0.0],
        }
    }

    /// Uploads the atlases on first use. Uploads are deferred, so this is
    /// safe to call from `update`.
    fn ensure_atlases(&mut self, gfx: &mut Graphics) {
        if self.spinner_atlas.is_none() {
            self.spinner_atlas = Some(gfx.upload_image(self.sheet.image()));
            log::info!(
                "uploaded spritesheet {}x{} ({} frames)",
                self.sheet.image().width(),
                self.sheet.image().height(),
                self.sheet.frame_count()
            );
        }
        if self.font_atlas.is_none() {
            self.font_atlas = Some(gfx.upload_image(self.font.atlas()));
        }
    }
}

impl Game for Demo {
    fn update(&mut self, dt: f32, input: &Input) {
        if input.just_pressed(Key::Space) {
            self.paused = !self.paused;
        }
        if input.just_pressed(Key::D) {
            self.speed = (self.speed * 1.5).min(6.0);
        }
        if input.just_pressed(Key::A) {
            self.speed = (self.speed / 1.5).max(0.1);
        }

        if !self.paused {
            self.elapsed_ms += dt as f64 * 1000.0 * self.speed as f64;
        }

        let mut dir = [0.0_f32, 0.0];
        if input.is_down(Key::Left) {
            dir[0] -= 1.0;
        }
        if input.is_down(Key::Right) {
            dir[0] += 1.0;
        }
        if input.is_down(Key::Up) {
            dir[1] += 1.0;
        }
        if input.is_down(Key::Down) {
            dir[1] -= 1.0;
        }
        self.player[0] = (self.player[0] + dir[0] * 4.0 * dt).clamp(-8.0, 8.0);
        self.player[1] = (self.player[1] + dir[1] * 4.0 * dt).clamp(-4.5, 4.5);
    }

    fn draw(&mut self, gfx: &mut Graphics) {
        self.ensure_atlases(gfx);

        gfx.camera.view_height = 12.0;
        gfx.camera.center = [0.0, 0.0];
        gfx.clear_color = [0.06, 0.07, 0.10];

        // Animate the spinner at the player's position.
        let spinner = self.spinner_atlas.unwrap();
        let frame = *self.animation.sample(self.elapsed_ms as u64);
        let px = self.sheet.frame_width() as f32 / 8.0;
        let py = self.sheet.frame_height() as f32 / 8.0;
        gfx.draw_frame(
            spinner,
            frame.rect,
            self.sheet.image().width(),
            self.sheet.image().height(),
            self.player,
            [px * 2.0, py * 2.0],
        );

        // Text overlay. World units, so 0.12 per font pixel is readable.
        let font = &self.font;
        let font_atlas = self.font_atlas.unwrap();
        gfx.draw_text(font, font_atlas, "FERRITE M4", [-5.5, 5.0], 0.12);
        gfx.draw_text(
            font,
            font_atlas,
            &format!("speed x{:.1}  {}", self.speed, if self.paused { "paused" } else { "playing" }),
            [-5.5, 4.2],
            0.09,
        );
        gfx.draw_text(
            font,
            font_atlas,
            "ARROWS move  SPACE pause  A/D speed",
            [-5.5, -5.2],
            0.09,
        );
    }
}

fn main() {
    env_logger::builder().format_timestamp_millis().init();
    run(Config::default(), Demo::new()).expect("event loop failed");
}
