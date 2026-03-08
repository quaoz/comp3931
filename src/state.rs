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
    settings::{CameraSettings, CullSettings, DisplaySettings, LodSettings, Settings},
    world::{World, scenes::hardcoded_scenes},
};

#[derive(Debug)]
pub enum Focus {
    World,
    Ui,
}

pub struct State {
    pub gui_renderer: EguiRenderer,
    pub renderer: Renderer,
    pub display: Display,
    pub world: World,

    pub settings: Settings,
    pub show_ui: bool,
    pub focus: Focus,

    fps_samples: VecDeque<f32>,
    debug_info: DebugInfo,

    last_draw_time: Instant,
    elapsed_secs: f32,
}

impl State {
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        let display = Display::new(window.clone()).await?;
        let renderer = Renderer::init(&display)?;
        let world = World::new(display.size());
        let egui_renderer = EguiRenderer::new(&display.device, display.format(), &window);
        let settings = Settings::default();

        Ok(Self {
            display,
            renderer,
            world,
            last_draw_time: Instant::now(),
            focus: Focus::Ui,
            gui_renderer: egui_renderer,
            settings,
            fps_samples: VecDeque::with_capacity(60),
            debug_info: DebugInfo::default(),
            show_ui: true,
            elapsed_secs: 0.0,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.display.resize(width, height);
        self.renderer.resize(&self.display.device, width, height);
        self.world.resize(width, height);
    }

    pub fn update(&mut self, dt: std::time::Duration) {
        self.elapsed_secs += dt.as_secs_f32();
        self.world.apply_settings(&self.settings);
        self.world.update(dt, &mut self.settings);
        self.renderer.update(
            &self.display,
            &mut self.world,
            &mut self.settings,
            self.elapsed_secs,
        );
    }

    pub fn draw(&mut self) {
        self.display.set_vsync(self.settings.display.vsync);

        // Frame rate cap, skip draw if not enough time has elapsed
        let frame_target = self.settings.display.frame_target;
        if frame_target > 0 {
            let min_frame = std::time::Duration::from_secs_f32(1.0 / frame_target as f32);
            if self.last_draw_time.elapsed() < min_frame {
                return;
            }
        }

        // FPS tracking
        let dt_secs = self.last_draw_time.elapsed().as_secs_f32();
        self.last_draw_time = Instant::now();
        if dt_secs > 0.0 {
            self.fps_samples.push_back(1.0 / dt_secs);
            if self.fps_samples.len() > 60 {
                self.fps_samples.pop_front();
            }

            let avg_fps = self.fps_samples.iter().sum::<f32>() / self.fps_samples.len() as f32;
            self.debug_info.frame_ms = dt_secs * 1000.0;
            self.debug_info.fps = avg_fps;

            self.debug_info.frame_history.push_back(dt_secs * 1000.0);
            if self.debug_info.frame_history.len() > 128 {
                self.debug_info.frame_history.pop_front();
            }
        }

        let (width, height) = self.display.size();
        if !self.display.is_surface_configured() {
            self.display.configure();
            self.world.resize(width, height);
            return;
        }

        let Some((frame, view)) = self
            .renderer
            .render_scene(&mut self.display, &self.settings.display)
        else {
            return;
        };

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: self.display.window().scale_factor() as f32,
        };

        let mut encoder = self
            .display
            .device
            .create_command_encoder(&Default::default());

        self.debug_info.mesh_index_count = self.renderer.mesh_index_count();
        self.debug_info.camera_pos = self.world.camera_position().to_array();
        self.debug_info.scene = self.world.scene_stats();

        // Keep a rolling history of non-zero rebuild costs for the rebuild bar chart.
        let rebuild_ms = self.debug_info.scene.last_rebuild_ms;
        if rebuild_ms > 0.0 {
            self.debug_info.rebuild_history.push_back(rebuild_ms);

            if self.debug_info.rebuild_history.len() > 64 {
                self.debug_info.rebuild_history.pop_front();
            }
        }

        let camera_info = self.world.camera_info();

        // TODO: cleanup, some of this should be wired directly
        self.gui_renderer.begin_frame(self.display.window());
        let actions = if self.show_ui {
            controls_ui(
                self.gui_renderer.context(),
                &mut self.settings,
                &self.debug_info,
                &camera_info,
            )
        } else {
            Default::default()
        };
        if actions.scene_dirty {
            self.settings.scene.active_mut().mark_dirty();
        }
        if let Some(seed) = actions.new_scene_seed {
            self.settings.scene.active_mut().seed = seed;
            self.settings.scene.active_mut().mark_dirty();
        }
        if actions.reset_scene {
            let active = self.settings.scene.active_scene;
            let old_eco_gen = self.settings.scene.scenes[active].generation;

            if let Some(mut default) = hardcoded_scenes()
                .into_iter()
                .find(|s| s.name == self.settings.scene.scenes[active].name)
            {
                default.generation = old_eco_gen.wrapping_add(1);
                default.mark_dirty();
                self.settings.scene.scenes[active] = default;
            }
        }
        if actions.save_scene {
            let mut new_scene = self.settings.scene.active().clone();
            new_scene.name = actions.save_scene_name;
            new_scene.mark_dirty();
            self.settings.scene.scenes.push(new_scene);
            self.settings.scene.active_scene = self.settings.scene.scenes.len() - 1;
        }
        if actions.reset_camera {
            self.settings.camera = CameraSettings::default();
            self.world.reset_camera();
        }
        if actions.set_orbit {
            let c = self.settings.camera.orbit_centre;
            self.world
                .camera
                .set_orbit(glam::Vec3::new(c[0], c[1], c[2]), 80.0);
        }
        if actions.set_fps {
            self.world.camera.set_fps();
        }
        if actions.reset_display {
            self.settings.display = DisplaySettings::default();
            self.settings.lod = LodSettings::default();
            self.settings.cull = CullSettings::default();
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

    pub fn render(&mut self, dt: std::time::Duration) {
        self.display.window().request_redraw();
        self.update(dt);
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
        if code == KeyCode::KeyU && pressed {
            self.show_ui = !self.show_ui;
            self.handle_focus(if self.show_ui {
                Focus::Ui
            } else {
                Focus::World
            });
            return;
        }

        if let Focus::World = self.focus {
            if code == KeyCode::Escape && pressed {
                self.handle_focus(Focus::Ui);
                return;
            }

            // Scene keyboard shortcuts
            if pressed {
                let step = self.settings.scene.progress_step as i32;
                let scene = self.settings.scene.active_mut();
                match code {
                    KeyCode::ArrowUp => {
                        scene.global_scale *= 1.1;
                        scene.mark_dirty();
                    }
                    KeyCode::ArrowDown => {
                        scene.global_scale *= 0.9;
                        scene.mark_dirty();
                    }
                    KeyCode::ArrowRight => {
                        scene.date = scene.date.advance_weeks(step);
                        scene.mark_dirty();
                    }
                    KeyCode::ArrowLeft => {
                        scene.date = scene.date.advance_weeks(-step);
                        scene.mark_dirty();
                    }
                    _ => {}
                }
            }

            self.world.handle_keyboard(code, pressed);
        }
    }
}
