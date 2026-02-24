use std::{collections::VecDeque, sync::Arc, time::Instant};

use anyhow::Result;
use egui_wgpu::ScreenDescriptor;
use winit::{
    event::{MouseScrollDelta, WindowEvent},
    keyboard::KeyCode,
    window::{CursorGrabMode, Window},
};

use crate::{
    graphics::{
        display::Display,
        gui::{DebugInfo, EguiRenderer, controls_ui},
        renderer::Renderer,
    },
    settings::{CameraSettings, DisplaySettings, LodSettings, Settings, season_needs_rebuild},
    world::{World, scenes::hardcoded_scenes},
};

#[derive(Debug)]
pub enum Focus {
    World,
    Ui,
}

pub struct State {
    pub focus: Focus,
    pub display: Display,
    pub renderer: Renderer,
    pub gui_renderer: EguiRenderer,
    pub last_render_time: Instant,
    last_draw_time: Instant,
    current_vsync: bool,
    pub world: World,
    pub settings: Settings,
    fps_samples: VecDeque<f32>,
    debug_info: DebugInfo,
}

impl State {
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        let display = Display::new(window.clone()).await?;
        let renderer = Renderer::init(&display)?;
        let world = World::new(display.surface_config.width, display.surface_config.height);
        let egui_renderer =
            EguiRenderer::new(&display.device, display.surface_config.format, &window);
        let settings = Settings::default();

        Ok(Self {
            display,
            renderer,
            world,
            last_render_time: Instant::now(),
            last_draw_time: Instant::now(),
            current_vsync: true,
            focus: Focus::Ui,
            gui_renderer: egui_renderer,
            settings,
            fps_samples: VecDeque::with_capacity(60),
            debug_info: DebugInfo::default(),
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

        // FPS tracking
        let dt_secs = dt.as_secs_f32();
        if dt_secs > 0.0 {
            self.fps_samples.push_back(1.0 / dt_secs);
            if self.fps_samples.len() > 60 {
                self.fps_samples.pop_front();
            }
            let avg_fps = self.fps_samples.iter().sum::<f32>() / self.fps_samples.len() as f32;
            self.debug_info.fps = avg_fps;
            self.debug_info.frame_ms = dt_secs * 1000.0;
            self.debug_info.frame_history.push_back(dt_secs * 1000.0);
            if self.debug_info.frame_history.len() > 128 {
                self.debug_info.frame_history.pop_front();
            }
        }

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
        let Some((frame, view)) =
            self.renderer
                .render_scene(&mut self.display, clear_color, show_lines, show_meshes)
        else {
            return;
        };

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [
                self.display.surface_config.width,
                self.display.surface_config.height,
            ],
            pixels_per_point: self.display.window().scale_factor() as f32,
        };

        let mut encoder = self
            .display
            .device
            .create_command_encoder(&Default::default());

        self.debug_info.mesh_index_count = self.renderer.mesh_index_count();

        self.gui_renderer.begin_frame(self.display.window());
        let actions = controls_ui(
            self.gui_renderer.context(),
            &mut self.settings,
            &self.debug_info,
        );
        if actions.reset_scene {
            let active = self.settings.scene.active_scene;
            let defaults = hardcoded_scenes();
            if let Some(default) = defaults
                .into_iter()
                .find(|s| s.name == self.settings.scene.scenes[active].name)
            {
                self.settings.scene.scenes[active] = default;
            }
        }
        if actions.save_scene {
            let mut new_scene = self.settings.scene.active().clone();
            new_scene.name = actions.save_scene_name;
            new_scene.dirty = true;
            self.settings.scene.scenes.push(new_scene);
            self.settings.scene.active_scene = self.settings.scene.scenes.len() - 1;
        }
        if actions.reset_camera {
            self.settings.camera = CameraSettings::default();
            self.world.reset_camera();
        }
        if actions.set_orbit {
            self.world.camera.set_orbit(glam::Vec3::ZERO, 80.0);
        }
        if actions.set_fps {
            self.world.camera.set_fps();
        }
        if actions.reset_display {
            self.settings.display = DisplaySettings::default();
            self.settings.lod = LodSettings::default();
        }
        self.gui_renderer.end_frame_and_draw(
            &self.display.device,
            &self.display.queue,
            &mut encoder,
            self.display.window(),
            &view,
            screen_descriptor,
        );

        self.display.queue.submit([encoder.finish()]);
        frame.present();
    }

    pub fn render(&mut self) {
        self.display.window().request_redraw();
        self.update();
        self.draw();
    }

    pub fn handle_focus(&mut self, focus: Focus) {
        match focus {
            Focus::Ui => {
                let window = self.display.window();
                window.set_cursor_grab(CursorGrabMode::None).ok();
                window.set_cursor_visible(true);
            }
            Focus::World => {
                let window = self.display.window();
                window
                    .set_cursor_grab(CursorGrabMode::Locked)
                    .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
                    .ok();
                window.set_cursor_visible(false);
            }
        }
        self.focus = focus;
    }

    pub fn handle_input(&mut self, event: &WindowEvent) -> bool {
        self.gui_renderer.handle_input(self.display.window(), event)
    }

    pub fn handle_mouse_motion(&mut self, dx: f64, dy: f64) {
        if let Focus::World = self.focus {
            self.world.handle_mouse_motion(dx, dy);
        };
    }

    pub fn handle_mouse_scroll(&mut self, delta: &MouseScrollDelta) {
        if let Focus::World = self.focus {
            self.world.handle_mouse_scroll(delta);
        };
    }

    pub fn handle_pinch(&mut self, delta: f64) {
        if let Focus::World = self.focus {
            self.world.handle_pinch(delta);
        }
    }

    pub fn handle_key(&mut self, code: KeyCode, pressed: bool) {
        if let Focus::World = self.focus {
            if code == KeyCode::Escape && pressed {
                self.handle_focus(Focus::Ui);
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
