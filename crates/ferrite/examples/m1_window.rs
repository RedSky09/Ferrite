use ferrite::{run, Config, Game, Graphics};

struct Demo {
    ticks: u64,
}

impl Game for Demo {
    fn update(&mut self, _dt: f32) {
        self.ticks += 1;
        // Log once per second (60 updates per second).
        if self.ticks % 60 == 0 {
            log::info!("tick {}", self.ticks);
        }
    }

    fn draw(&mut self, _gfx: &mut Graphics) {}
}

fn main() {
    // Millisecond timestamps make timing issues easier to spot.
    env_logger::builder().format_timestamp_millis().init();
    run(Config::default(), Demo { ticks: 0 }).expect("event loop failed");
}
