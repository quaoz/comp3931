use std::{fmt::Debug, time::Duration};

use anyhow::Result;
use winit::keyboard::KeyCode;

use crate::framework::display::Display;

pub trait Renderer: 'static + Sized + Send + Debug {
    fn init(display: &Display) -> Result<Self>;

    fn resize(&mut self, display: &Display);
    fn update(&mut self, display: &Display, dt: Duration);
    fn render(&mut self, display: &mut Display);

    #[allow(unused)]
    fn handle_keyboard(&mut self, key: KeyCode, pressed: bool) {}
    #[allow(unused)]
    fn handle_mouse_move(&mut self, dx: f64, dy: f64) {}
    #[allow(unused)]
    fn handle_mouse_button(&mut self, button: u32, pressed: bool) {}
}
