use winit::event_loop::EventLoop;

use crate::app::{App, AppEvent};

pub mod app;
pub mod graphics;
pub mod settings;
pub mod state;
pub mod util;
pub mod world;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::<AppEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();

    let mut app = App::new(proxy);
    event_loop.run_app(&mut app)?;

    Ok(())
}
