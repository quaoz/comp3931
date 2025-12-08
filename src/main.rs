mod app;
mod framework;
mod turtle;
mod util;

use winit::event_loop::EventLoop;

use crate::{app::App, turtle::renderer::TurtleRenderer};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = App::<TurtleRenderer>::new(&event_loop);
    event_loop.run_app(&mut app)?;

    Ok(())
}
