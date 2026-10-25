use ferrite::{run, Config, Game};

struct Demo {
    ticks: u64,
}

impl Game for Demo {
    fn update(&mut self, _dt: f32) {
        self.ticks += 1;
        if self.ticks % 60 == 0 {
            log::info!("tick {}", self.ticks);
        }
    }

    fn draw(&mut self) {}
}

fn main() {
    env_logger::init();
    run(Config::default(), Demo { ticks: 0 }).expect("event loop gagal");
}
