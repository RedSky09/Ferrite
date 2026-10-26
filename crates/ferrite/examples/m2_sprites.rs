use ferrite::{run, Config, Game, Graphics, Sprite};

/// Draws a wavy grid of 960 sprites and sways the camera.
struct Demo {
    time: f32,
}

impl Game for Demo {
    fn update(&mut self, dt: f32) {
        self.time += dt;
    }

    fn draw(&mut self, gfx: &mut Graphics) {
        let t = self.time;

        gfx.camera.view_height = 24.0;
        gfx.camera.center = [(t * 0.5).sin() * 3.0, 0.0];

        for gx in -20..20 {
            for gy in -12..12 {
                let (fx, fy) = (gx as f32, gy as f32);
                let wave = 0.25 * (t * 2.0 + fx * 0.4).sin();
                let color = [
                    0.5 + 0.5 * (t + fx * 0.2).sin(),
                    0.5 + 0.5 * (t * 1.3 + fy * 0.3).sin(),
                    1.0,
                ];
                gfx.draw_sprite(Sprite::new([fx, fy + wave], [0.8, 0.8]).with_color(color));
            }
        }
    }
}

fn main() {
    // Millisecond timestamps make timing issues easier to spot.
    env_logger::builder().format_timestamp_millis().init();
    run(Config::default(), Demo { time: 0.0 }).expect("event loop failed");
}
