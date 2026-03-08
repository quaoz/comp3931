use std::collections::VecDeque;

use egui::{CollapsingHeader, ComboBox, Context, DragValue, Grid, Slider};
use egui_wgpu::{
    Renderer, RendererOptions, ScreenDescriptor, wgpu,
    wgpu::{CommandEncoder, Device, Queue, StoreOp, TextureFormat, TextureView},
};
use winit::{event::WindowEvent, window::Window};

use crate::{
    settings::{EnvironmentSettings, PlantType, Settings},
    world::{CameraInfo, plants::PlantInstance, scenes::SceneStats},
};

#[derive(Default, Clone)]
pub struct DebugInfo {
    pub fps: f32,
    pub frame_ms: f32,
    pub mesh_index_count: u32,
    pub frame_history: VecDeque<f32>,
    pub rebuild_history: VecDeque<f32>,
    pub camera_pos: [f32; 3],
    pub scene: SceneStats,
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

    pub fn new(device: &Device, output_colour_format: TextureFormat, window: &Window) -> Self {
        let egui_context = Context::default();

        let egui_state = egui_winit::State::new(
            egui_context,
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            Some(2 * 1024),
        );
        let egui_renderer = Renderer::new(device, output_colour_format, RendererOptions::default());

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
    pub scene_dirty: bool,
    pub new_scene_seed: Option<u64>,
}

pub fn controls_ui(
    ctx: &Context,
    settings: &mut Settings,
    debug: &DebugInfo,
    camera_info: &CameraInfo,
) -> UiActions {
    let mut actions = UiActions::default();

    egui::SidePanel::right("control_panel")
        .default_width(250.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                scene_ui(ui, settings, &mut actions);
                ui.separator();
                environment_ui(ui, &mut settings.env);
                ui.separator();
                camera_ui(ui, settings, camera_info, &mut actions);
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
            CollapsingHeader::new("Plants")
                .default_open(false)
                .show(ui, |ui| {
                    plant_list_ui(ui, settings);
                });
        });
}

fn scene_selector_ui(ui: &mut egui::Ui, settings: &mut Settings, actions: &mut UiActions) {
    // ── Selector ──────────────────────────────────────────────────────────────
    let prev_active = settings.scene.active_scene;
    let scene_names: Vec<String> = settings
        .scene
        .scenes
        .iter()
        .map(|s| s.name.clone())
        .collect();
    let mut new_active = prev_active;

    ui.horizontal(|ui| {
        ComboBox::from_id_salt("scene_choose_combo")
            .selected_text(&scene_names[prev_active])
            .show_ui(ui, |ui| {
                for (i, name) in scene_names.iter().enumerate() {
                    ui.selectable_value(&mut new_active, i, name);
                }
            });

        if settings.scene.is_hardcoded(settings.scene.active()) && ui.button("Reset").clicked() {
            actions.reset_scene = true;
        }
    });

    settings.scene.active_scene = new_active;
    if settings.scene.active_scene != prev_active {
        settings.scene.active_mut().mark_dirty();
    }

    // ── Save ──────────────────────────────────────────────────────────────────
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

    ui.separator();

    // ── Seed ──────────────────────────────────────────────────────────────────
    let mut dirty = false;
    Grid::new("scene_seed_grid")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            let scene = settings.scene.active_mut();

            ui.label("Scale");
            dirty |= ui
                .add(Slider::new(&mut scene.global_scale, 0.1..=10.0).logarithmic(true))
                .changed();
            ui.end_row();

            ui.label("Seed");
            ui.horizontal(|ui| {
                let old_seed = scene.seed;
                if ui.add(DragValue::new(&mut scene.seed).speed(1.0)).changed()
                    && scene.seed != old_seed
                {
                    dirty = true;
                }
                if ui
                    .small_button("⟳")
                    .on_hover_text("Randomise seed")
                    .clicked()
                {
                    actions.new_scene_seed = Some(rand::random());
                }
            });
            ui.end_row();

            if dirty {
                scene.mark_dirty();
            }
        });

    ui.separator();

    // ── Date ──────────────────────────────────────────────────────────────────
    let sc = &mut settings.scene;
    let mut date_dirty = false;
    Grid::new("scene_date_grid")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            let scene = sc.active_mut();

            ui.label("Year");
            date_dirty |= ui
                .add(DragValue::new(&mut scene.date.year).range(0..=1000))
                .changed();
            ui.end_row();

            ui.label("Week");
            date_dirty |= ui
                .add(DragValue::new(&mut scene.date.week).range(0..=51))
                .changed();
            ui.end_row();

            // Season dropdown — shows current season and jumps to the first week when changed.
            ui.label("Season");
            const SEASON_NAMES: [&str; 4] = ["Winter", "Spring", "Summer", "Autumn"];
            const SEASON_START: [u32; 4] = [0, 13, 26, 39];
            let cur_season_idx = (scene.date.week / 13).min(3) as usize;
            let mut sel_season = cur_season_idx;
            ComboBox::from_id_salt("scene_season_combo")
                .selected_text(SEASON_NAMES[cur_season_idx])
                .show_ui(ui, |ui| {
                    for (i, &name) in SEASON_NAMES.iter().enumerate() {
                        ui.selectable_value(&mut sel_season, i, name);
                    }
                });
            if sel_season != cur_season_idx {
                scene.date.week = SEASON_START[sel_season];
                date_dirty = true;
            }
            ui.end_row();

            if date_dirty {
                scene.mark_dirty();
            }
        });

    ui.separator();

    // ── Auto-progress ─────────────────────────────────────────────────────────
    Grid::new("scene_progress_grid")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            ui.label("Auto-progress");
            ui.checkbox(&mut sc.auto_progress, "");
            ui.end_row();

            ui.label("Rate (steps/s)");
            ui.add(
                DragValue::new(&mut sc.progress_rate)
                    .range(1.0..=60.0)
                    .speed(0.1)
                    .fixed_decimals(2),
            );
            ui.end_row();

            ui.label("Step size (wk)");
            ui.add(DragValue::new(&mut sc.progress_step).range(1..=52));
            ui.end_row();

            ui.label("Max steps")
                .on_hover_text("Stop auto-progress after this many steps. 0 = no limit.");
            ui.horizontal(|ui| {
                ui.add(
                    DragValue::new(&mut sc.max_steps)
                        .range(0..=u32::MAX)
                        .speed(1.0),
                );
                if sc.max_steps == 0 {
                    ui.weak("∞");
                } else if sc.steps_taken > 0 {
                    ui.weak(format!("{}/{}", sc.steps_taken, sc.max_steps));
                }
            });
            ui.end_row();
        });
}

fn plant_list_ui(ui: &mut egui::Ui, settings: &mut Settings) {
    let mut remove_idx = None;
    let mut dirty = false;

    let scene = settings.scene.active_mut();
    for (i, plant) in scene.plants.iter_mut().enumerate() {
        ui.collapsing(format!("{} ({})", plant.plant.plant_type(), i + 1), |ui| {
            ui.horizontal(|ui| {
                ui.label("Delay (yr)").on_hover_text(
                    "Random delay in years before this plant starts growing. At year 0 week 0 all \
                     plants have iteration 0.",
                );
                dirty |= ui
                    .add(
                        DragValue::new(&mut plant.delay_years)
                            .range(0.0..=5.0)
                            .speed(0.1)
                            .fixed_decimals(1),
                    )
                    .changed();
            });

            CollapsingHeader::new("Transform")
                .default_open(false)
                .show(ui, |ui| {
                    Grid::new(format!("plant_{i}_xform"))
                        .num_columns(2)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("Scale");
                            dirty |= ui
                                .add(Slider::new(&mut plant.scale, 0.1..=50.0).logarithmic(true))
                                .changed();
                            ui.end_row();

                            ui.label("Rotation");
                            dirty |= ui
                                .add(Slider::new(&mut plant.rotation, 0.0..=360.0).suffix("°"))
                                .changed();
                            ui.end_row();

                            ui.label("Position");
                            ui.horizontal(|ui| {
                                dirty |= ui
                                    .add(
                                        DragValue::new(&mut plant.position.x)
                                            .prefix("x: ")
                                            .speed(0.5),
                                    )
                                    .changed();
                                dirty |= ui
                                    .add(
                                        DragValue::new(&mut plant.position.y)
                                            .prefix("y: ")
                                            .speed(0.5),
                                    )
                                    .changed();
                                dirty |= ui
                                    .add(
                                        DragValue::new(&mut plant.position.z)
                                            .prefix("z: ")
                                            .speed(0.5),
                                    )
                                    .changed();
                            });
                            ui.end_row();
                        });
                });

            CollapsingHeader::new("Parameters")
                .default_open(true)
                .show(ui, |ui| {
                    dirty |= plant.plant.ui(ui);
                });

            if ui.button("Remove").clicked() {
                remove_idx = Some(i);
            }
        });
    }

    if dirty {
        scene.mark_dirty();
    }
    if let Some(idx) = remove_idx {
        scene.plants.remove(idx);
        scene.mark_dirty();
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
            .push(PlantInstance::new(plant_type, 0.0));
        settings.scene.active_mut().mark_dirty();
    }
}

fn environment_ui(ui: &mut egui::Ui, env: &mut EnvironmentSettings) {
    CollapsingHeader::new("Environment")
        .default_open(true)
        .show(ui, |ui| {
            // Growth & Tropism
            ui.strong("Growth & Tropism");
            Grid::new("env_growth_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Phototropism").on_hover_text(
                        "Bends branches toward the light source. Higher values produce more \
                         pronounced lean toward the sun.",
                    );
                    if ui
                        .add(Slider::new(&mut env.tropism_strength, 0.0..=1.0))
                        .changed()
                    {
                        env.mark_dirty();
                    }
                    ui.end_row();

                    ui.label("Gravitropism").on_hover_text(
                        "Pulls branches downward (negative gravitropism bends them upward). \
                         Models gravity's effect on branch angle.",
                    );
                    if ui
                        .add(Slider::new(&mut env.gravitropism_strength, 0.0..=1.0))
                        .changed()
                    {
                        env.mark_dirty();
                    }
                    ui.end_row();

                    ui.label("Branch taper").on_hover_text(
                        "How much branch radius shrinks at each internode. 1.0 = no taper; lower \
                         values produce more conical branches.",
                    );
                    if ui.add(Slider::new(&mut env.taper, 0.3..=1.0)).changed() {
                        env.mark_dirty();
                    }
                    ui.end_row();

                    ui.label("Self-straightening").on_hover_text(
                        "How quickly shoots correct back toward vertical after bending. Higher = \
                         stiffer stems that resist deflection.",
                    );
                    if ui
                        .add(
                            Slider::new(&mut env.proprioception_gamma, 0.0..=0.5).fixed_decimals(3),
                        )
                        .changed()
                    {
                        env.mark_dirty();
                    }
                    ui.end_row();

                    ui.label("Branch drift").on_hover_text(
                        "Amplitude of random shape variation along each branch. Higher = more \
                         organic, irregular silhouettes.",
                    );
                    if ui
                        .add(
                            Slider::new(&mut env.variation_noise_strength, 0.0..=0.2)
                                .fixed_decimals(3),
                        )
                        .changed()
                    {
                        env.mark_dirty();
                    }
                    ui.end_row();

                    ui.label("Drift memory").on_hover_text(
                        "How far drift persists along a branch before reverting to straight. Low \
                         = long sweeping curves; high = rapid short wiggles.",
                    );
                    if ui
                        .add(
                            Slider::new(&mut env.variation_decay_rate, 0.0..=1.0).fixed_decimals(2),
                        )
                        .changed()
                    {
                        env.mark_dirty();
                    }
                    ui.end_row();

                    ui.label("Colour variation").on_hover_text(
                        "Per-branch colour drift amplitude. Adds subtle hue variation across the \
                         canopy for a more natural look.",
                    );
                    if ui
                        .add(Slider::new(&mut env.colour_variation, 0.0..=0.15).fixed_decimals(3))
                        .changed()
                    {
                        env.mark_dirty();
                    }
                    ui.end_row();
                });

            // Wind
            ui.strong("Wind");
            Grid::new("env_wind_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Direction").on_hover_text(
                        "Compass bearing the wind blows from (0° = north, 90° = east).",
                    );
                    if ui
                        .add(Slider::new(&mut env.wind_azimuth, 0.0..=360.0).suffix("°"))
                        .changed()
                    {
                        env.mark_dirty();
                    }
                    ui.end_row();

                    ui.label("Strength").on_hover_text(
                        "Overall wind force. Deflects branches in the wind direction at geometry \
                         build time.",
                    );
                    if ui
                        .add(Slider::new(&mut env.wind_strength, 0.0..=1.0))
                        .changed()
                    {
                        env.mark_dirty();
                    }
                    ui.end_row();

                    ui.label("Turbulence").on_hover_text(
                        "Random variation layered on top of the mean wind direction. Higher = \
                         gustier, less uniform deflection.",
                    );
                    if ui
                        .add(Slider::new(&mut env.wind_turbulence, 0.0..=0.3))
                        .changed()
                    {
                        env.mark_dirty();
                    }
                    ui.end_row();

                    ui.label("Anim strength").on_hover_text(
                        "Vertex-shader sway amplitude applied every frame. Set to 0 to freeze \
                         geometry for screenshots. Uses the same direction as baked wind.",
                    );
                    ui.add(Slider::new(&mut env.wind_anim_strength, 0.0..=0.1).max_decimals(3));
                    ui.end_row();
                });

            // Light & Appearance
            ui.strong("Light & Appearance");
            Grid::new("env_light_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Light position").on_hover_text(
                        "World-space position of the directional light source used for shading \
                         and phototropism.",
                    );
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
                            env.mark_dirty();
                        }
                    });
                    ui.end_row();

                    ui.label("Ambient").on_hover_text(
                        "Minimum light level applied to all surfaces, simulating indirect \
                         skylight. 0 = fully dark shadows; 1 = flat unlit look.",
                    );
                    if ui.add(Slider::new(&mut env.ambient, 0.0..=1.0)).changed() {
                        env.mark_dirty();
                    }
                    ui.end_row();
                });

            if ui.button("Reset").clicked() {
                *env = EnvironmentSettings::default();
                env.mark_dirty();
            }
        });
}

fn camera_ui(
    ui: &mut egui::Ui,
    settings: &mut Settings,
    camera_info: &CameraInfo,
    actions: &mut UiActions,
) {
    ui.collapsing("Camera", |ui| {
        Grid::new("camera_grid")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                ui.label("FOV");
                ui.add(Slider::new(&mut settings.camera.fov, 30.0..=120.0).suffix("°"));
                ui.end_row();

                ui.label("Speed");
                ui.add(Slider::new(&mut settings.camera.speed, 1.0..=100.0));
                ui.end_row();

                ui.label("Sensitivity");
                ui.add(
                    Slider::new(&mut settings.camera.sensitivity, 0.0001..=0.01).logarithmic(true),
                );
                ui.end_row();
            });

        if camera_info.is_orbit {
            Grid::new("camera_auto_orbit_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Orbit centre");
                    ui.horizontal(|ui| {
                        ui.add(
                            DragValue::new(&mut settings.camera.orbit_centre[0])
                                .prefix("x: ")
                                .speed(1.0),
                        )
                        .changed();
                        ui.add(
                            DragValue::new(&mut settings.camera.orbit_centre[1])
                                .prefix("y: ")
                                .speed(1.0),
                        )
                        .changed();
                        ui.add(
                            DragValue::new(&mut settings.camera.orbit_centre[2])
                                .prefix("z: ")
                                .speed(1.0),
                        )
                        .changed();
                    });
                    ui.end_row();

                    ui.label("Orbit speed");
                    ui.add(
                        Slider::new(&mut settings.camera.orbit_speed, 0.05..=5.0).logarithmic(true),
                    );
                    ui.end_row();

                    ui.label("Auto-orbit");
                    ui.checkbox(&mut settings.camera.auto_orbit, "");
                    ui.end_row();
                });
        }

        ui.horizontal(|ui| {
            if ui.button("Orbit").clicked() {
                actions.set_orbit = true;
                settings.camera.auto_orbit = false;
            }
            if ui.button("FPS").clicked() {
                actions.set_fps = true;
                settings.camera.auto_orbit = false;
            }
            if ui.button("Reset").clicked() {
                actions.reset_camera = true;
                settings.camera.auto_orbit = false;
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
                ui.label("Background");
                ui.color_edit_button_rgb(&mut settings.display.background_colour);
                ui.end_row();

                ui.label("Ground");
                ui.color_edit_button_rgb(&mut settings.display.ground_colour);
                ui.end_row();

                ui.label("Lines");
                ui.checkbox(&mut settings.display.show_lines, "");
                ui.end_row();

                ui.label("Meshes");
                ui.checkbox(&mut settings.display.show_meshes, "");
                ui.end_row();

                ui.label("Debug HUD");
                if ui.checkbox(&mut settings.display.debug_mode, "").changed() {
                    actions.scene_dirty = true;
                }
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
                    ui.label("Near (full)").on_hover_text(
                        "Distance threshold for full-detail rendering (8 cylinder segments).",
                    );
                    actions.scene_dirty |= ui
                        .add(Slider::new(
                            &mut settings.lod.near_threshold,
                            10.0..=settings.lod.mid_threshold,
                        ))
                        .changed();
                    ui.end_row();

                    ui.label("Mid (medium)").on_hover_text(
                        "Distance threshold for medium-detail rendering (5 cylinder segments).",
                    );
                    actions.scene_dirty |= ui
                        .add(Slider::new(
                            &mut settings.lod.mid_threshold,
                            settings.lod.near_threshold..=settings.lod.far_threshold,
                        ))
                        .changed();
                    ui.end_row();

                    ui.label("Far (low)").on_hover_text(
                        "Distance threshold for low-detail rendering (3 cylinder segments). \
                         Plants beyond this distance skip mesh generation.",
                    );
                    actions.scene_dirty |= ui
                        .add(Slider::new(
                            &mut settings.lod.far_threshold,
                            settings.lod.mid_threshold..=2000.0,
                        ))
                        .changed();
                    ui.end_row();

                    ui.label("Hysteresis").on_hover_text(
                        "Dead-zone around each LOD boundary. Prevents rapid geometry rebuilds \
                         when a plant sits right on a distance threshold.",
                    );
                    actions.scene_dirty |= ui
                        .add(Slider::new(&mut settings.cull.lod_hysteresis, 0.0..=20.0))
                        .changed();
                    ui.end_row();

                    ui.label("Leaf skip (near)").on_hover_text(
                        "Fraction of leaf quads randomly skipped at near LOD distance. 0 = off.",
                    );
                    actions.scene_dirty |= ui
                        .add(
                            egui::Slider::new(&mut settings.lod.leaf_skip_near, 0.0..=1.0)
                                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                                .custom_parser(|s| {
                                    s.trim_end_matches('%').parse().ok().map(|v: f64| v / 100.0)
                                }),
                        )
                        .changed();
                    ui.end_row();

                    ui.label("Leaf skip (mid)").on_hover_text(
                        "Fraction of leaf quads randomly skipped at mid LOD distance. 0 = off.",
                    );
                    actions.scene_dirty |= ui
                        .add(
                            egui::Slider::new(&mut settings.lod.leaf_skip_mid, 0.0..=1.0)
                                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                                .custom_parser(|s| {
                                    s.trim_end_matches('%').parse().ok().map(|v: f64| v / 100.0)
                                }),
                        )
                        .changed();
                    ui.end_row();

                    ui.label("Leaf skip (far)").on_hover_text(
                        "Fraction of leaf quads randomly skipped at far LOD distance. 0 = off.",
                    );
                    actions.scene_dirty |= ui
                        .add(
                            egui::Slider::new(&mut settings.lod.leaf_skip_far, 0.0..=1.0)
                                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                                .custom_parser(|s| {
                                    s.trim_end_matches('%').parse().ok().map(|v: f64| v / 100.0)
                                }),
                        )
                        .changed();
                    ui.end_row();
                });
        });

        if ui.button("Reset").clicked() {
            actions.reset_display = true;
        }
    });
}

fn mini_bar_graph(
    ui: &mut egui::Ui,
    history: &std::collections::VecDeque<f32>,
    max_val: f32,
    width: f32,
    height: f32,
    thresholds: &[(f32, egui::Color32)],
    default_colour: egui::Color32,
) {
    let graph_size = egui::Vec2::new(width, height);
    let (response, painter) = ui.allocate_painter(graph_size, egui::Sense::hover());
    let rect = response.rect;
    painter.rect_filled(rect, 2.0, egui::Color32::from_black_alpha(120));
    if history.is_empty() {
        return;
    }
    let n = history.len();
    let bar_w = rect.width() / n as f32;
    for (i, &val) in history.iter().enumerate() {
        let h = (val / max_val).min(1.0) * rect.height();
        let x = rect.left() + i as f32 * bar_w;
        let colour = thresholds
            .iter()
            .find(|&&(t, _)| val > t)
            .map(|&(_, c)| c)
            .unwrap_or(default_colour);
        painter.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(x, rect.bottom() - h),
                egui::Vec2::new(bar_w.max(1.0), h),
            ),
            0.0,
            colour,
        );
    }
    for &(thresh, _) in thresholds {
        let y = rect.bottom() - (thresh / max_val).min(1.0) * rect.height();
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(1.0, egui::Color32::from_gray(180)),
        );
    }
}

fn debug_ui(ctx: &Context, debug: &DebugInfo) {
    egui::Window::new("Debug")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::LEFT_TOP, [8.0, 8.0])
        .show(ctx, |ui| {
            // ── Frame timing ──
            ui.label(format!("FPS: {:.0}  ({:.2} ms)", debug.fps, debug.frame_ms));
            mini_bar_graph(
                ui,
                &debug.frame_history,
                50.0,
                200.0,
                48.0,
                &[
                    (33.3, egui::Color32::from_rgb(220, 80, 80)),
                    (16.7, egui::Color32::from_rgb(220, 200, 60)),
                ],
                egui::Color32::from_rgb(80, 200, 100),
            );

            // ── Geometry ──
            ui.separator();
            ui.label(format!("Triangles: {}", debug.mesh_index_count / 3));
            ui.label(format!("Line verts: {}", debug.scene.line_vertex_count));

            // ── Scene / LOD ──
            ui.separator();
            ui.label(format!("Plants: {}", debug.scene.plant_count));

            let tc = &debug.scene.lod_tier_counts;
            let [near, mid, far] = debug.scene.lod_thresholds.map(|v| v as u32);
            egui::Grid::new("lod_tier_grid")
                .num_columns(3)
                .spacing([6.0, 1.0])
                .show(ui, |ui| {
                    ui.label("Near");
                    ui.label(format!("<{}m", near));
                    ui.label(format!("{}", tc[0]));
                    ui.end_row();

                    ui.label("Mid");
                    ui.label(format!("{}\u{2013}{}m", near, mid));
                    ui.label(format!("{}", tc[1]));
                    ui.end_row();

                    ui.label("Far");
                    ui.label(format!("{}\u{2013}{}m", mid, far));
                    ui.label(format!("{}", tc[2]));
                    ui.end_row();

                    ui.label("Beyond");
                    ui.label(format!(">{}", far));
                    ui.label(format!("{}", tc[3]));
                    ui.end_row();

                    ui.label("Culled");
                    ui.label("");
                    ui.label(format!("{}", debug.scene.culled_count));
                    ui.end_row();
                });

            if debug.scene.lod_pressure < 0.999 {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 180, 50),
                    format!("pressure {:.2}", debug.scene.lod_pressure),
                );
            }

            // ── Scene rebuilds ──
            ui.separator();
            if debug.scene.last_rebuild_ms > 0.0 {
                let kind = if debug.scene.last_rebuild_full {
                    "full"
                } else {
                    "partial"
                };
                ui.label(format!(
                    "Build: {:.1} ms ({})",
                    debug.scene.last_rebuild_ms, kind
                ));
            } else {
                ui.label("Build: cached");
            }
            if !debug.rebuild_history.is_empty() {
                let max_rebuild = debug
                    .rebuild_history
                    .iter()
                    .cloned()
                    .fold(0.0_f32, f32::max)
                    .max(10.0);
                mini_bar_graph(
                    ui,
                    &debug.rebuild_history,
                    max_rebuild,
                    200.0,
                    32.0,
                    &[],
                    egui::Color32::from_rgb(120, 180, 240),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "Last {} rebuilds (max {:.0} ms)",
                        debug.rebuild_history.len(),
                        max_rebuild
                    ))
                    .small(),
                );
            }

            // ── Camera ──
            ui.separator();
            let [x, y, z] = debug.camera_pos;
            ui.label(format!("Camera: ({x:.1}, {y:.1}, {z:.1})"));
        });
}
