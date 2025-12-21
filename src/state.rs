use std::{sync::Arc, time::Instant};

use anyhow::Result;
use winit::{
    event::MouseScrollDelta,
    keyboard::KeyCode,
    window::{CursorGrabMode, Window},
};

use crate::{
    graphics::{display::Display, renderer::Renderer},
    world::World,
};

#[derive(Debug)]
pub struct State {
    pub display: Display,
    pub renderer: Renderer,
    pub world: World,
    pub last_render_time: Instant,
    pub focused: bool,
}

impl State {
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        let display = Display::new(window).await?;
        let renderer = Renderer::init(&display)?;
        let world = World::new(display.surface_config.width, display.surface_config.height);

        Ok(Self {
            display,
            renderer,
            world,
            last_render_time: Instant::now(),
            focused: false,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.display.resize(width, height);
        self.renderer.resize(&self.display.device, width, height);
        self.world.resize(width, height);
    }

    pub fn render(&mut self) {
        self.display.window().request_redraw();

        let dt = self.last_render_time.elapsed();
        self.last_render_time = Instant::now();

        self.world.update(dt);
        self.renderer.update(&self.display, &mut self.world);

        if self.display.is_surface_configured() {
            self.renderer.render(&mut self.display);
        } else {
            self.display.configure();

            let (w, h) = self.display.size();
            self.world.resize(w, h);
        };
    }

    pub fn set_cursor_captured(&mut self, captured: bool) {
        let window = self.display.window();
        if captured {
            window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
                .ok();
            window.set_cursor_visible(false);
        } else {
            window.set_cursor_grab(CursorGrabMode::None).ok();
            window.set_cursor_visible(true);
        }
        self.focused = captured;
    }

    pub fn handle_mouse_motion(&mut self, dx: f64, dy: f64) {
        if self.focused {
            self.world.handle_mouse_motion(dx, dy);
        }
    }

    pub fn handle_mouse_scroll(&mut self, delta: &MouseScrollDelta) {
        self.world.handle_mouse_scroll(delta);
    }

    pub fn handle_mouse_button() {}

    pub fn handle_key(&mut self, code: KeyCode, pressed: bool) {
        if self.focused {
            if code == KeyCode::Escape && pressed {
                self.set_cursor_captured(false);
            } else {
                self.world.handle_keyboard(code, pressed);
            }
        }
    }
}
