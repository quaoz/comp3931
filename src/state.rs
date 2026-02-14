use std::{sync::Arc, time::Instant};

use anyhow::Result;
use winit::{
    event::MouseScrollDelta,
    keyboard::KeyCode,
    window::{CursorGrabMode, Window},
};

use crate::{
    graphics::{display::Display, renderer::Renderer},
    settings::{Settings, season_needs_rebuild},
    world::World,
};

pub struct State {
    pub display: Display,
    pub renderer: Renderer,
    pub world: World,
    pub last_render_time: Instant,
    pub focused: bool,
    pub settings: Settings,
    last_draw_time: Instant,
    current_vsync: bool,
}

impl State {
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        let display = Display::new(window.clone()).await?;
        let renderer = Renderer::init(&display)?;
        let world = World::new(display.surface_config.width, display.surface_config.height);
        let settings = Settings::default();

        Ok(Self {
            display,
            renderer,
            world,
            last_render_time: Instant::now(),
            last_draw_time: Instant::now(),
            current_vsync: true,
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

        // Season auto-advance: tint updates every frame via shader, geometry only when age changes
        if self.settings.env.auto_advance {
            let old = self.settings.env.season;
            let new = (old + self.settings.env.season_speed * dt.as_secs_f32()).rem_euclid(1.0);
            self.settings.env.season = new;
            if season_needs_rebuild(old, new) {
                self.settings.env.dirty = true;
            }
        }

        self.world.apply_settings(&self.settings);
        self.world.update(dt);
        self.renderer
            .update(&self.display, &mut self.world, &mut self.settings);
    }

    pub fn draw(&mut self) {
        // Apply vsync setting changes
        let want_vsync = self.settings.display.vsync;
        if want_vsync != self.current_vsync {
            self.display.set_vsync(want_vsync);
            self.current_vsync = want_vsync;
        }

        // Frame rate cap: skip draw if not enough time has elapsed
        let frame_target = self.settings.display.frame_target;
        if frame_target > 0 {
            let min_frame = std::time::Duration::from_secs_f32(1.0 / frame_target as f32);
            if self.last_draw_time.elapsed() < min_frame {
                return;
            }
        }
        self.last_draw_time = Instant::now();

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

    pub fn handle_pinch(&mut self, delta: f64) {
        if self.focused {
            self.world.handle_pinch(delta);
        }
    }

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
