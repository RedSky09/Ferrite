use std::sync::Arc;

use ferrite_core::time::FixedTimestep;
use ferrite_render::Renderer;
use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

pub struct Config {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub updates_per_second: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            title: "Ferrite".into(),
            width: 960,
            height: 540,
            updates_per_second: 60,
        }
    }
}

pub trait Game {
    /// Dipanggil dengan langkah waktu tetap (detik).
    fn update(&mut self, dt: f32);
    /// Dipanggil sekali per frame.
    fn draw(&mut self);
}

struct App<G: Game> {
    config: Config,
    game: G,
    timer: FixedTimestep,
    // Arc karena wgpu di M2 butuh berbagi kepemilikan window.
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
}

impl<G: Game> ApplicationHandler for App<G> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.config.width,
                self.config.height,
            ));
        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("gagal membuat window"),
        );
        let renderer = Renderer::new(window.clone(), event_loop.owned_display_handle());
        self.window = Some(window);
        self.renderer = Some(renderer);
        log::info!("window dan renderer siap");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                let steps = self.timer.advance();
                for _ in 0..steps {
                    self.game.update(self.timer.step_secs());
                }
                self.game.draw();
                if let Some(renderer) = &mut self.renderer {
                    renderer.render();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Minta frame berikutnya terus-menerus.
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

pub fn run<G: Game>(config: Config, game: G) -> Result<(), EventLoopError> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App {
        timer: FixedTimestep::new(config.updates_per_second),
        config,
        game,
        window: None,
        renderer: None,
    };
    event_loop.run_app(&mut app)
}
