use winit::application::ApplicationHandler;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes};

struct App {
    window: Option<Window>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("resumed!");
        if self.window.is_none() {
            self.window = Some(
                event_loop
                    .create_window(WindowAttributes::default())
                    .unwrap(),
            );
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        println!("window_event {:?} {:?} {:?}", event_loop, window_id, event);
        match event {
            winit::event::WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        };
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    let _ = event_loop.run_app(&mut App { window: None });
}
