use ferrite::{run, Config, Game, Graphics, Input, Key, Sprite};

/// Move a square with WASD or the arrow keys. Space toggles its color.
struct Demo {
    pos: [f32; 2],
    highlighted: bool,
}

impl Game for Demo {
    fn update(&mut self, dt: f32, input: &Input) {
        let mut dir = [0.0_f32, 0.0];
        if input.is_down(Key::Left) || input.is_down(Key::A) {
            dir[0] -= 1.0;
        }
        if input.is_down(Key::Right) || input.is_down(Key::D) {
            dir[0] += 1.0;
        }
        if input.is_down(Key::Down) || input.is_down(Key::S) {
            dir[1] -= 1.0;
        }
        if input.is_down(Key::Up) || input.is_down(Key::W) {
            dir[1] += 1.0;
        }

        // Normalize so moving diagonally is not faster than moving straight.
        let len = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt();
        if len > 0.0 {
            let speed = 6.0; // world units per second
            self.pos[0] += dir[0] / len * speed * dt;
            self.pos[1] += dir[1] / len * speed * dt;
        }

        // A single press, no matter how long the key is held.
        if input.just_pressed(Key::Space) {
            self.highlighted = !self.highlighted;
            log::info!("highlighted = {}", self.highlighted);
        }
    }

    fn draw(&mut self, gfx: &mut Graphics) {
        gfx.camera.view_height = 12.0;
        gfx.camera.center = self.pos; // the camera follows the player

        // A dim grid so movement is easy to see.
        for gx in -10..=10 {
            for gy in -10..=10 {
                let center = [gx as f32 * 2.0, gy as f32 * 2.0];
                gfx.draw_sprite(Sprite::new(center, [1.6, 1.6]).with_color([0.35, 0.40, 0.50]));
            }
        }

        // The player, drawn last so it appears on top.
        let color = if self.highlighted {
            [1.0, 0.6, 0.2]
        } else {
            [1.0, 1.0, 1.0]
        };
        gfx.draw_sprite(Sprite::new(self.pos, [1.0, 1.0]).with_color(color));
    }
}

fn main() {
    // Millisecond timestamps make timing issues easier to spot.
    env_logger::builder().format_timestamp_millis().init();
    run(
        Config::default(),
        Demo {
            pos: [0.0, 0.0],
            highlighted: false,
        },
    )
    .expect("event loop failed");
}
