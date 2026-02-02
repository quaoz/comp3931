use std::{sync::Arc, time::Instant};

use anyhow::Result;
use winit::{
    event::MouseScrollDelta,
    keyboard::KeyCode,
    window::{CursorGrabMode, Window},
};

use crate::{
    graphics::{display::Display, renderer::Renderer},
    settings::Settings,
    world::World,
};

pub struct State {
    pub display: Display,
    pub renderer: Renderer,
    pub world: World,
    pub last_render_time: Instant,
    pub focused: bool,
    pub settings: Settings,
}

impl State {
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        let display = Display::new(window).await?;
        let renderer = Renderer::init(&display)?;
        let world = World::new(display.surface_config.width, display.surface_config.height);
        let settings = Settings::default();

        Ok(Self {
            display,
            renderer,
            world,
            last_render_time: Instant::now(),
            focused: false,
            settings,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.display.resize(width, height);
        self.renderer.resize(&self.display.device, width, height);
        self.world.resize(width, height);
    }

    pub fn update(&mut self) {
        let dt = self.last_render_time.elapsed();
        self.last_render_time = Instant::now();

        self.world.apply_settings(&self.settings);
        self.world.update(dt);
        self.renderer
            .update(&self.display, &mut self.world, &mut self.settings);
    }

    pub fn draw(&mut self) {
        if !self.display.is_surface_configured() {
            self.display.configure();
            let (w, h) = self.display.size();
            self.world.resize(w, h);
            return;
        }

        let clear_color = self.settings.display.clear_color();

        let show_lines = self.settings.display.show_lines;
        let show_meshes = self.settings.display.show_meshes;
        let Some((frame, _)) =
            self.renderer
                .render_scene(&mut self.display, clear_color, show_lines, show_meshes)
        else {
            return;
        };

        let encoder = self
            .display
            .device
            .create_command_encoder(&Default::default());

        self.display.queue.submit([encoder.finish()]);
        frame.present();
    }

    pub fn render(&mut self) {
        self.display.window().request_redraw();
        self.update();
        self.draw();
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
                return;
            }

            // Scene keyboard shortcuts
            if pressed {
                let scene = self.settings.scene.active_mut();
                match code {
                    KeyCode::ArrowUp => {
                        scene.global_scale *= 1.1;
                        scene.dirty = true;
                    }
                    KeyCode::ArrowDown => {
                        scene.global_scale *= 0.9;
                        scene.dirty = true;
                    }
                    KeyCode::ArrowRight => {
                        scene.global_age += 1;
                        scene.dirty = true;
                    }
                    KeyCode::ArrowLeft => {
                        scene.global_age = scene.global_age.saturating_sub(1);
                        scene.dirty = true;
                    }
                    _ => {}
                }
            }

            self.world.handle_keyboard(code, pressed);
        }
    }
}
