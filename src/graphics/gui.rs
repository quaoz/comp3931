use std::collections::VecDeque;

use egui::{CollapsingHeader, ComboBox, Context, DragValue, Grid, Slider};
use egui_wgpu::{
    Renderer, RendererOptions, ScreenDescriptor, wgpu,
    wgpu::{CommandEncoder, Device, Queue, StoreOp, TextureFormat, TextureView},
};
use winit::{event::WindowEvent, window::Window};

use crate::{
    settings::{EnvironmentSettings, PlantType, Settings, season_name},
    world::plants::PlantInstance,
};

#[derive(Default, Clone)]
pub struct DebugInfo {
    pub fps: f32,
    pub frame_ms: f32,
    pub mesh_index_count: u32,
    /// Rolling history of frame times (ms), newest at back, max 128 entries.
    pub frame_history: VecDeque<f32>,
}

pub struct EguiRenderer {
    state: egui_winit::State,
    renderer: Renderer,
    frame_started: bool,
}

impl EguiRenderer {
    pub fn context(&self) -> &Context {
        self.state.egui_ctx()
    }

    pub fn new(device: &Device, output_color_format: TextureFormat, window: &Window) -> Self {
        let egui_context = Context::default();

        let egui_state = egui_winit::State::new(
            egui_context,
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            Some(2 * 1024),
        );
        let egui_renderer = Renderer::new(device, output_color_format, RendererOptions::default());

        Self {
            state: egui_state,
            renderer: egui_renderer,
            frame_started: false,
        }
    }

    pub fn handle_input(&mut self, window: &Window, event: &WindowEvent) -> bool {
        self.state.on_window_event(window, event).consumed
    }

    pub fn begin_frame(&mut self, window: &Window) {
        let raw_input = self.state.take_egui_input(window);
        self.state.egui_ctx().begin_pass(raw_input);
        self.frame_started = true;
    }

    pub fn end_frame_and_draw(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &Window,
        window_surface_view: &TextureView,
        screen_descriptor: ScreenDescriptor,
    ) {
        if !self.frame_started {
            panic!("begin_frame must be called before end_frame_and_draw can be called!");
        }

        self.state
            .egui_ctx()
            .set_pixels_per_point(screen_descriptor.pixels_per_point);

        let full_output = self.state.egui_ctx().end_pass();

        self.state
            .handle_platform_output(window, full_output.platform_output);

        let tris = self
            .state
            .egui_ctx()
            .tessellate(full_output.shapes, self.state.egui_ctx().pixels_per_point());
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }
        self.renderer
            .update_buffers(device, queue, encoder, &tris, &screen_descriptor);
        let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: window_surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            label: Some("egui render pass"),
            occlusion_query_set: None,
        });

        self.renderer
            .render(&mut rpass.forget_lifetime(), &tris, &screen_descriptor);
        for x in &full_output.textures_delta.free {
            self.renderer.free_texture(x)
        }

        self.frame_started = false;
    }
}

#[derive(Default)]
pub struct UiActions {
    pub reset_scene: bool,
    pub reset_camera: bool,
    pub reset_display: bool,
    pub save_scene: bool,
    pub save_scene_name: String,
    pub set_orbit: bool,
    pub set_fps: bool,
}

pub fn controls_ui(ctx: &Context, settings: &mut Settings, debug: &DebugInfo) -> UiActions {
    let mut actions = UiActions::default();

    egui::SidePanel::right("control_panel")
        .default_width(250.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                scene_ui(ui, settings, &mut actions);
                ui.separator();
                environment_ui(ui, &mut settings.env);
                ui.separator();
                camera_ui(ui, settings, &mut actions);
                ui.separator();
                display_ui(ui, settings, &mut actions);
            });
        });

    if settings.display.debug_mode {
        debug_ui(ctx, debug);
    }

    actions
}

fn scene_ui(ui: &mut egui::Ui, settings: &mut Settings, actions: &mut UiActions) {
    CollapsingHeader::new("Scene")
        .default_open(true)
        .show(ui, |ui| {
            scene_selector_ui(ui, settings, actions);
            ui.separator();
            ui.label("Plants");
            plant_list_ui(ui, settings);
        });
}

fn scene_selector_ui(ui: &mut egui::Ui, settings: &mut Settings, actions: &mut UiActions) {
    let scene_names: Vec<String> = settings
        .scene
        .scenes
        .iter()
        .map(|s| s.name.clone())
        .collect();
    let prev_active = settings.scene.active_scene;

    Grid::new("scene_grid")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            ui.label("Scene");
            ComboBox::from_id_salt("scene_choose_combo")
                .selected_text(&scene_names[prev_active])
                .show_ui(ui, |ui| {
                    for (i, name) in scene_names.iter().enumerate() {
                        ui.selectable_value(&mut settings.scene.active_scene, i, name);
                    }
                });
            ui.end_row();

            if settings.scene.active_scene != prev_active {
                settings.scene.active_mut().dirty = true;
            }

            let scene = settings.scene.active_mut();
            ui.label("Scale");
            if ui
                .add(Slider::new(&mut scene.global_scale, 0.1..=10.0).logarithmic(true))
                .changed()
            {
                scene.dirty = true;
            }
            ui.end_row();

            ui.label("Age");
            if ui.add(Slider::new(&mut scene.global_age, 0..=20)).changed() {
                scene.dirty = true;
            }
            ui.end_row();
        });

    // save scene with name input
    let save_name_id = ui.make_persistent_id("save_scene_name");
    let mut save_name: String = ui
        .ctx()
        .data_mut(|d| d.get_temp(save_name_id).unwrap_or_default());

    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut save_name)
                .hint_text(format!("Scene {}", settings.scene.scenes.len() + 1))
                .desired_width(120.0),
        );

        if ui.button("Save").clicked() {
            let name = if save_name.trim().is_empty() {
                format!("Scene {}", settings.scene.scenes.len() + 1)
            } else {
                save_name.trim().to_string()
            };
            actions.save_scene = true;
            actions.save_scene_name = name;
            save_name = String::new();
        }
    });
    ui.ctx()
        .data_mut(|d| d.insert_temp(save_name_id, save_name));

    if settings.scene.is_hardcoded(settings.scene.active()) && ui.button("Reset").clicked() {
        actions.reset_scene = true;
    }
}

fn plant_list_ui(ui: &mut egui::Ui, settings: &mut Settings) {
    let mut remove_idx = None;
    let mut dirty = false;

    let scene = settings.scene.active_mut();
    for (i, plant) in scene.plants.iter_mut().enumerate() {
        ui.collapsing(format!("{} ({})", plant.plant.plant_type(), i + 1), |ui| {
            Grid::new(format!("plant_{i}_grid"))
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Age");
                    dirty |= ui
                        .add(Slider::new(&mut plant.base_age, 0..=plant.plant.max_age()))
                        .changed();
                    ui.end_row();

                    ui.label("Scale");
                    dirty |= ui
                        .add(Slider::new(&mut plant.scale, 0.1..=50.0).logarithmic(true))
                        .changed();
                    ui.end_row();

                    ui.label("Rotation");
                    dirty |= ui
                        .add(Slider::new(&mut plant.rotation, 0.0..=360.0))
                        .changed();
                    ui.end_row();

                    ui.label("Position:");
                    ui.horizontal(|ui| {
                        dirty |= ui
                            .add(
                                DragValue::new(&mut plant.position[0])
                                    .prefix("x: ")
                                    .speed(0.5),
                            )
                            .changed();
                        dirty |= ui
                            .add(
                                DragValue::new(&mut plant.position[1])
                                    .prefix("y: ")
                                    .speed(0.5),
                            )
                            .changed();
                        dirty |= ui
                            .add(
                                DragValue::new(&mut plant.position[2])
                                    .prefix("z: ")
                                    .speed(0.5),
                            )
                            .changed();
                    });
                    ui.end_row();
                });

            // TODO: pre plant ui

            if ui.button("Remove").clicked() {
                remove_idx = Some(i);
            }
        });
    }

    if dirty {
        scene.dirty = true;
    }
    if let Some(idx) = remove_idx {
        let scene = settings.scene.active_mut();
        scene.plants.remove(idx);
        scene.dirty = true;
    }

    let mut selected: Option<PlantType> = None;
    ComboBox::from_id_salt("plant_add_combo")
        .selected_text("Add Plant")
        .show_ui(ui, |ui| {
            for t in PlantType::ALL {
                ui.selectable_value(&mut selected, Some(t), t.to_string());
            }
        });

    if let Some(plant_type) = selected {
        settings
            .scene
            .active_mut()
            .plants
            .push(PlantInstance::new(plant_type, 4));
        settings.scene.active_mut().dirty = true;
    }
}

fn environment_ui(ui: &mut egui::Ui, env: &mut EnvironmentSettings) {
    CollapsingHeader::new("Environment")
        .default_open(true)
        .show(ui, |ui| {
            Grid::new("env_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label(format!("Season ({})", season_name(env.season)));
                    if ui
                        .add(Slider::new(&mut env.season, 0.0..=1.0).step_by(0.01))
                        .changed()
                    {
                        env.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Auto-advance");
                    ui.checkbox(&mut env.auto_advance, "");
                    ui.end_row();

                    ui.label("Season speed");
                    ui.add(Slider::new(&mut env.season_speed, 0.001..=0.2).logarithmic(true));
                    ui.end_row();

                    ui.label("Phototropism");
                    if ui
                        .add(Slider::new(&mut env.tropism_strength, 0.0..=1.0))
                        .changed()
                    {
                        env.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Gravitropism");
                    if ui
                        .add(Slider::new(&mut env.gravitropism_strength, 0.0..=1.0))
                        .changed()
                    {
                        env.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Wind dir (°)");
                    if ui
                        .add(Slider::new(&mut env.wind_azimuth, 0.0..=360.0))
                        .changed()
                    {
                        env.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Wind strength");
                    if ui
                        .add(Slider::new(&mut env.wind_strength, 0.0..=1.0))
                        .changed()
                    {
                        env.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Turbulence");
                    if ui
                        .add(Slider::new(&mut env.wind_turbulence, 0.0..=0.3))
                        .changed()
                    {
                        env.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Branch taper");
                    if ui.add(Slider::new(&mut env.taper, 0.3..=1.0)).changed() {
                        env.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Light");
                    ui.horizontal(|ui| {
                        let mut changed = false;
                        changed |= ui
                            .add(
                                DragValue::new(&mut env.light_position[0])
                                    .prefix("x: ")
                                    .speed(1.0),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                DragValue::new(&mut env.light_position[1])
                                    .prefix("y: ")
                                    .speed(1.0),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                DragValue::new(&mut env.light_position[2])
                                    .prefix("z: ")
                                    .speed(1.0),
                            )
                            .changed();
                        if changed {
                            env.dirty = true;
                        }
                    });
                    ui.end_row();

                    ui.label("Ambient");
                    if ui.add(Slider::new(&mut env.ambient, 0.0..=1.0)).changed() {
                        env.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Seed");
                    if ui.add(DragValue::new(&mut env.seed).speed(1.0)).changed() {
                        env.dirty = true;
                    }
                    ui.end_row();
                });

            ui.horizontal(|ui| {
                if ui.button("Reseed").clicked() {
                    env.seed = rand::random();
                    env.dirty = true;
                }
                if ui.button("Reset").clicked() {
                    *env = EnvironmentSettings::default();
                    env.dirty = true;
                }
            });
        });
}

fn camera_ui(ui: &mut egui::Ui, settings: &mut Settings, actions: &mut UiActions) {
    ui.collapsing("Camera", |ui| {
        Grid::new("camera_grid")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                ui.label("Speed");
                ui.add(Slider::new(&mut settings.camera.speed, 1.0..=100.0));
                ui.end_row();

                ui.label("Sensitivity");
                ui.add(
                    Slider::new(&mut settings.camera.sensitivity, 0.0001..=0.01).logarithmic(true),
                );
                ui.end_row();
            });

        ui.horizontal(|ui| {
            if ui.button("Orbit").clicked() {
                actions.set_orbit = true;
            }
            if ui.button("FPS").clicked() {
                actions.set_fps = true;
            }
            if ui.button("Reset").clicked() {
                actions.reset_camera = true;
            }
        });
    });
}

fn display_ui(ui: &mut egui::Ui, settings: &mut Settings, actions: &mut UiActions) {
    ui.collapsing("Display", |ui| {
        Grid::new("display_grid")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                ui.label("FOV");
                ui.add(Slider::new(&mut settings.display.fov, 30.0..=120.0));
                ui.end_row();

                ui.label("Background");
                ui.color_edit_button_rgb(&mut settings.display.background_color);
                ui.end_row();

                ui.label("Ground");
                ui.color_edit_button_rgb(&mut settings.display.ground_color);
                ui.end_row();

                ui.label("Lines");
                ui.checkbox(&mut settings.display.show_lines, "");
                ui.end_row();

                ui.label("Meshes");
                ui.checkbox(&mut settings.display.show_meshes, "");
                ui.end_row();

                ui.label("Debug HUD");
                ui.checkbox(&mut settings.display.debug_mode, "");
                ui.end_row();

                ui.label("VSync");
                ui.checkbox(&mut settings.display.vsync, "");
                ui.end_row();

                ui.label("Frame limit");
                ui.horizontal(|ui| {
                    ui.add(
                        DragValue::new(&mut settings.display.frame_target)
                            .range(0..=480)
                            .speed(1.0),
                    );
                    ui.label(if settings.display.frame_target == 0 {
                        "unlimited"
                    } else {
                        "fps"
                    });
                });
                ui.end_row();
            });

        ui.collapsing("LOD", |ui| {
            Grid::new("lod_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Near (full)");
                    ui.add(Slider::new(&mut settings.lod.near_threshold, 10.0..=200.0));
                    ui.end_row();

                    ui.label("Mid (medium)");
                    ui.add(Slider::new(&mut settings.lod.mid_threshold, 50.0..=500.0));
                    ui.end_row();

                    ui.label("Far (low)");
                    ui.add(Slider::new(&mut settings.lod.far_threshold, 100.0..=1000.0));
                    ui.end_row();

                    ui.label("Max indices");
                    ui.horizontal(|ui| {
                        ui.add(
                            DragValue::new(&mut settings.lod.max_indices)
                                .range(0..=u32::MAX)
                                .speed(10000.0),
                        );
                        ui.label(if settings.lod.max_indices == 0 {
                            "off"
                        } else {
                            ""
                        });
                    });
                    ui.end_row();
                });
        });

        if ui.button("Reset").clicked() {
            actions.reset_display = true;
        }
    });
}

fn debug_ui(ctx: &Context, debug: &DebugInfo) {
    egui::Window::new("Debug")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::LEFT_TOP, [8.0, 8.0])
        .show(ctx, |ui| {
            ui.label(format!("FPS: {:.0}", debug.fps));
            ui.label(format!("Frame: {:.2} ms", debug.frame_ms));
            ui.label(format!("Triangles: {}", debug.mesh_index_count / 3));

            // Frame time graph
            let graph_size = egui::Vec2::new(200.0, 48.0);
            let (response, painter) = ui.allocate_painter(graph_size, egui::Sense::hover());
            let rect = response.rect;

            painter.rect_filled(rect, 2.0, egui::Color32::from_black_alpha(120));

            let max_ms = 50.0_f32;
            let history = &debug.frame_history;
            if !history.is_empty() {
                let n = history.len();
                let bar_w = rect.width() / n as f32;
                for (i, &ms) in history.iter().enumerate() {
                    let h = (ms / max_ms).min(1.0) * rect.height();
                    let x = rect.left() + i as f32 * bar_w;
                    let color = if ms > 33.3 {
                        egui::Color32::from_rgb(220, 80, 80)
                    } else if ms > 16.7 {
                        egui::Color32::from_rgb(220, 200, 60)
                    } else {
                        egui::Color32::from_rgb(80, 200, 100)
                    };
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(x, rect.bottom() - h),
                            egui::Vec2::new(bar_w.max(1.0), h),
                        ),
                        0.0,
                        color,
                    );
                }
                // Reference lines at 60fps (16.7ms) and 30fps (33.3ms)
                for thresh in [16.7f32, 33.3] {
                    let y = rect.bottom() - (thresh / max_ms) * rect.height();
                    painter.line_segment(
                        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                        egui::Stroke::new(1.0, egui::Color32::from_gray(180)),
                    );
                }
            }
        });
}
