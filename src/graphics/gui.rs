use std::collections::VecDeque;

use egui::{CollapsingHeader, ComboBox, Context, DragValue, Grid, Slider};
use egui_wgpu::{
    Renderer, RendererOptions, ScreenDescriptor, wgpu,
    wgpu::{CommandEncoder, Device, Queue, StoreOp, TextureFormat, TextureView},
};
use winit::{event::WindowEvent, window::Window};

use crate::{
    perf::{PerfLogger, RebuildBreakdown},
    settings::{EcosystemKernel, EnvironmentSettings, PlantType, SceneData, Settings},
    util::widget,
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
    perf: &mut PerfLogger,
) -> UiActions {
    let mut actions = UiActions::default();

    egui::SidePanel::right("control_panel")
        .default_width(250.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                scene_ui(ui, settings, &mut actions, debug);
                ui.separator();

                environment_ui(ui, &mut settings.env);
                ui.separator();

                camera_ui(ui, settings, camera_info, &mut actions);
                ui.separator();

                display_ui(ui, settings, &mut actions);

                // Recording status badge
                if perf.is_recording() {
                    ui.separator();
                    ui.colored_label(
                        egui::Color32::from_rgb(220, 60, 60),
                        format!("● REC  {:.0} s", perf.elapsed_secs()),
                    );
                }
            });
        });

    if settings.display.debug_mode {
        debug_ui(ctx, debug, perf);
    }

    actions
}

fn scene_ui(
    ui: &mut egui::Ui,
    settings: &mut Settings,
    actions: &mut UiActions,
    debug: &DebugInfo,
) {
    CollapsingHeader::new("Scene")
        .default_open(true)
        .show(ui, |ui| {
            scene_selector_ui(ui, settings, actions, debug);
            ui.separator();

            ecosystem_ui(ui, settings.scene.active_mut());
            ui.separator();

            CollapsingHeader::new("Plants")
                .default_open(false)
                .show(ui, |ui| {
                    plant_list_ui(ui, settings);
                });
        });
}

fn scene_selector_ui(
    ui: &mut egui::Ui,
    settings: &mut Settings,
    actions: &mut UiActions,
    _debug: &DebugInfo,
) {
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
                    scene.ecosystem.mark_dirty();
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

            // Season dropdown
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

fn ecosystem_ui(ui: &mut egui::Ui, scene: &mut SceneData) {
    let eco = &mut scene.ecosystem;
    CollapsingHeader::new("Ecosystem")
        .default_open(true)
        .show(ui, |ui| {
            let mut dirty = false;

            ui.strong("Placement");
            Grid::new("ecosystem_placement_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    dirty |= widget::row_hover(
                        ui,
                        "Area",
                        "Side length of the square region plants are scattered across",
                        Slider::new(&mut eco.area, 20.0..=300.0),
                    );
                    dirty |= widget::row_hover(
                        ui,
                        "Plants",
                        "Total number of plants to place in the scene.",
                        Slider::new(&mut eco.num_plants, 1..=3000),
                    );

                    let prev_kernel = eco.kernel;
                    widget::custom_row_hover(
                        ui,
                        "Kernel",
                        "Spatial interaction kernel applied during placement:\n• Neutral — purely \
                         random \n• Inhibitory — plants repel each other \n• Promotional — plants \
                         attract each other \n•  Mixed — short-range inhibition, long-range \
                         attraction",
                        |ui| {
                            ComboBox::from_id_salt("eco_kernel_combo")
                                .selected_text(eco.kernel.to_string())
                                .show_ui(ui, |ui| {
                                    for k in EcosystemKernel::ALL {
                                        ui.selectable_value(&mut eco.kernel, k, k.to_string());
                                    }
                                });
                        },
                    );
                    dirty |= eco.kernel != prev_kernel;

                    dirty |= widget::row_hover(
                        ui,
                        "Kernel radius",
                        "Distance over which the spatial kernel acts. Larger values produce \
                         broader clustering or exclusion zones.",
                        Slider::new(&mut eco.kernel_radius, 1.0..=50.0).logarithmic(true),
                    );
                });

            ui.strong("Species");
            let mut remove_idx: Option<usize> = None;
            for (i, (plant_type, weight)) in eco.species.iter_mut().enumerate() {
                let prev_type = *plant_type;
                ui.horizontal(|ui| {
                    ComboBox::from_id_salt(format!("eco_sp_{i}"))
                        .selected_text(plant_type.to_string())
                        .width(90.0)
                        .show_ui(ui, |ui| {
                            for t in PlantType::ALL {
                                ui.selectable_value(plant_type, t, t.to_string());
                            }
                        });
                    dirty |= ui
                        .add(
                            Slider::new(weight, 0.1..=5.0)
                                .text("weight")
                                .fixed_decimals(1),
                        )
                        .on_hover_text(format!(
                            "Relative abundance weight for placement sampling.\nShade tolerance: \
                             {:.2}\nGrowth rate: {:.2}",
                            plant_type.shade_tolerance(),
                            plant_type.growth_rate(),
                        ))
                        .changed();
                    if ui.small_button("x").clicked() {
                        remove_idx = Some(i);
                    }
                });
                dirty |= *plant_type != prev_type;
            }
            if let Some(i) = remove_idx {
                eco.species.remove(i);
                dirty = true;
            }

            let mut add_species: Option<PlantType> = None;
            ComboBox::from_id_salt("eco_add_species_combo")
                .selected_text("Add species")
                .show_ui(ui, |ui| {
                    for t in PlantType::ALL {
                        ui.selectable_value(&mut add_species, Some(t), t.to_string());
                    }
                });
            if let Some(t) = add_species {
                eco.species.push((t, 1.0));
                dirty = true;
            }

            ui.strong("Self-Thinning");
            Grid::new("ecosystem_thinning_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    dirty |= widget::check_row(
                        ui,
                        "Enabled",
                        "Remove plants that are too close together, simulating competition for \
                         resources. Smaller plants are removed first.",
                        &mut eco.use_self_thinning,
                    );
                    dirty |= widget::row_hover(
                        ui,
                        "Thinning radius",
                        "Minimum allowed distance between plants. Plants closer than this may be \
                         culled.",
                        Slider::new(&mut eco.thinning_radius, 1.0..=30.0),
                    );
                });

            ui.strong("Succession");
            Grid::new("ecosystem_succession_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    dirty |= widget::check_row(
                        ui,
                        "Enabled",
                        "Iteratively replace shade-intolerant species with more tolerant ones in \
                         densely shaded areas, simulating long-term vegetation change.",
                        &mut eco.use_succession,
                    );
                    dirty |= widget::row_hover(
                        ui,
                        "Steps",
                        "Number of succession iterations to simulate. More steps produce a more \
                         mature, climax community.",
                        Slider::new(&mut eco.succession_steps, 1..=30),
                    );
                });

            if dirty {
                eco.mark_dirty();
            }
        });
}

fn environment_ui(ui: &mut egui::Ui, env: &mut EnvironmentSettings) {
    CollapsingHeader::new("Environment")
        .default_open(true)
        .show(ui, |ui| {
            let mut dirty = false;

            ui.strong("Growth & Tropism");
            Grid::new("env_growth_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    dirty |= widget::row_hover(
                        ui,
                        "Phototropism",
                        "Bends branches toward the light source. Higher values produce more \
                         pronounced lean toward the sun.",
                        Slider::new(&mut env.tropism_strength, 0.0..=1.0),
                    );
                    dirty |= widget::row_hover(
                        ui,
                        "Gravitropism",
                        "Pulls branches downward (negative gravitropism bends them upward). \
                         Models gravity's effect on branch angle.",
                        Slider::new(&mut env.gravitropism_strength, 0.0..=1.0),
                    );
                    dirty |= widget::row_hover(
                        ui,
                        "Branch taper",
                        "How much branch radius shrinks at each internode. 1.0 = no taper; lower \
                         values produce more conical branches.",
                        Slider::new(&mut env.taper, 0.3..=1.0),
                    );
                    dirty |= widget::row_hover(
                        ui,
                        "Self-straightening",
                        "How quickly shoots correct back toward vertical after bending. Higher = \
                         stiffer stems that resist deflection.",
                        Slider::new(&mut env.proprioception_gamma, 0.0..=0.5).fixed_decimals(3),
                    );
                    dirty |= widget::row_hover(
                        ui,
                        "Branch drift",
                        "Amplitude of random shape variation along each branch. Higher = more \
                         organic, irregular silhouettes.",
                        Slider::new(&mut env.variation_noise_strength, 0.0..=0.2).fixed_decimals(3),
                    );
                    dirty |= widget::row_hover(
                        ui,
                        "Drift memory",
                        "How far drift persists along a branch before reverting to straight. Low \
                         = long sweeping curves; high = rapid short wiggles.",
                        Slider::new(&mut env.variation_decay_rate, 0.0..=1.0).fixed_decimals(2),
                    );
                    dirty |= widget::row_hover(
                        ui,
                        "Colour variation",
                        "Per-branch colour drift amplitude. Adds subtle hue variation across the \
                         canopy for a more natural look.",
                        Slider::new(&mut env.colour_variation, 0.0..=0.15).fixed_decimals(3),
                    );
                });

            ui.strong("Wind");
            Grid::new("env_wind_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    dirty |= widget::row_hover(
                        ui,
                        "Direction",
                        "Compass bearing the wind blows from (0° = north, 90° = east).",
                        Slider::new(&mut env.wind_azimuth, 0.0..=360.0).suffix("°"),
                    );
                    dirty |= widget::row_hover(
                        ui,
                        "Strength",
                        "Overall wind force. Deflects branches in the wind direction at geometry \
                         build time.",
                        Slider::new(&mut env.wind_strength, 0.0..=1.0),
                    );
                    dirty |= widget::row_hover(
                        ui,
                        "Turbulence",
                        "Random variation layered on top of the mean wind direction. Higher = \
                         gustier, less uniform deflection.",
                        Slider::new(&mut env.wind_turbulence, 0.0..=0.3),
                    );
                    // Animated sway is a shader effect — it needs no geometry rebuild.
                    widget::row_hover(
                        ui,
                        "Anim strength",
                        "Vertex-shader sway amplitude applied every frame.",
                        Slider::new(&mut env.wind_anim_strength, 0.0..=0.1).max_decimals(3),
                    );
                });

            ui.strong("Space Pruning");
            Grid::new("env_pruning_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    dirty |= widget::check_row(
                        ui,
                        "Enabled",
                        "Cut branches whose tip enters a voxel cell already occupied by another \
                         branch. Produces natural crown shaping and prevents interpenetration \
                         between neighbouring plants.",
                        &mut env.space_pruning,
                    );
                    if env.space_pruning {
                        dirty |= widget::row_hover(
                            ui,
                            "Cell size",
                            "Voxel resolution in world units. Smaller = finer pruning and denser \
                             packing; larger = only coarse overlap is detected.",
                            Slider::new(&mut env.occupancy_cell_size, 0.1..=2.0).fixed_decimals(2),
                        );
                    }
                });

            ui.strong("Light & Appearance");
            Grid::new("env_light_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    dirty |= widget::vec3_row(
                        ui,
                        "Light position",
                        "World-space position of the directional light source used for shading \
                         and phototropism.",
                        &mut env.light_position,
                        1.0,
                    );
                    dirty |= widget::row_hover(
                        ui,
                        "Ambient",
                        "Minimum light level applied to all surfaces, simulating indirect \
                         skylight. 0 = fully dark shadows; 1 = flat unlit look.",
                        Slider::new(&mut env.ambient, 0.0..=1.0),
                    );
                });

            if ui.button("Reset").clicked() {
                *env = EnvironmentSettings::default();
                dirty = true;
            }

            if dirty {
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
        let cam = &mut settings.camera;

        Grid::new("camera_grid")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                widget::row(
                    ui,
                    "FOV",
                    Slider::new(&mut cam.fov, 30.0..=120.0).suffix("°"),
                );
                widget::row(ui, "Speed", Slider::new(&mut cam.speed, 1.0..=100.0));
                widget::row(
                    ui,
                    "Sensitivity",
                    Slider::new(&mut cam.sensitivity, 0.0001..=0.01).logarithmic(true),
                );
            });

        if camera_info.is_orbit {
            Grid::new("camera_auto_orbit_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    widget::vec3_row(ui, "Orbit centre", "", &mut cam.orbit_centre, 1.0);
                    widget::row(
                        ui,
                        "Orbit speed",
                        Slider::new(&mut cam.orbit_speed, 0.05..=5.0).logarithmic(true),
                    );
                    widget::check_row(ui, "Auto-orbit", "", &mut cam.auto_orbit);
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

/// Slider over a 0–1 fraction, displayed and entered as a percentage.
fn percent_slider(value: &mut f32) -> Slider<'_> {
    Slider::new(value, 0.0..=1.0)
        .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
        .custom_parser(|s| s.trim_end_matches('%').parse().ok().map(|v: f64| v / 100.0))
}

fn display_ui(ui: &mut egui::Ui, settings: &mut Settings, actions: &mut UiActions) {
    ui.collapsing("Display", |ui| {
        Grid::new("display_grid")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                let display = &mut settings.display;

                widget::rgb_row(ui, "Background", &mut display.background_colour);
                widget::rgb_row(ui, "Ground", &mut display.ground_colour);
                widget::check_row(ui, "Lines", "", &mut display.show_lines);
                widget::check_row(ui, "Meshes", "", &mut display.show_meshes);
                actions.scene_dirty |=
                    widget::check_row(ui, "Debug HUD", "", &mut display.debug_mode);
                widget::check_row(ui, "VSync", "", &mut display.vsync);

                widget::custom_row(ui, "Frame limit", |ui| {
                    ui.add(
                        DragValue::new(&mut display.frame_target)
                            .range(0..=480)
                            .speed(1.0),
                    );
                    ui.label(if display.frame_target == 0 {
                        "unlimited"
                    } else {
                        "fps"
                    });
                });
            });

        ui.collapsing("Culling", |ui| {
            Grid::new("cull_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    actions.scene_dirty |= widget::check_row(
                        ui,
                        "Frustum culling",
                        "Skip plants that are entirely outside the camera's view frustum. Disable \
                         if plants incorrectly disappear at screen edges.",
                        &mut settings.cull.frustum_culling,
                    );
                    if settings.cull.frustum_culling {
                        actions.scene_dirty |= widget::row_hover(
                            ui,
                            "Cull radius",
                            "Bounding-sphere radius used for frustum culling (world units). \
                             Increase if large plants are incorrectly culled near the screen edge.",
                            Slider::new(&mut settings.cull.cull_radius, 1.0..=100.0).suffix(" m"),
                        );
                    }
                });
        });

        ui.collapsing("LOD", |ui| {
            Grid::new("lod_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    // Each threshold is clamped by its neighbours so the tiers stay ordered.
                    let (near, mid, far) = (
                        settings.lod.near_threshold,
                        settings.lod.mid_threshold,
                        settings.lod.far_threshold,
                    );

                    actions.scene_dirty |= widget::row_hover(
                        ui,
                        "Near (full)",
                        "Distance threshold for full-detail rendering (8 cylinder segments).",
                        Slider::new(&mut settings.lod.near_threshold, 10.0..=mid),
                    );
                    actions.scene_dirty |= widget::row_hover(
                        ui,
                        "Mid (medium)",
                        "Distance threshold for medium-detail rendering (5 cylinder segments).",
                        Slider::new(&mut settings.lod.mid_threshold, near..=far),
                    );
                    actions.scene_dirty |= widget::row_hover(
                        ui,
                        "Far (low)",
                        "Distance threshold for low-detail rendering (3 cylinder segments). \
                         Plants beyond this distance skip mesh generation.",
                        Slider::new(&mut settings.lod.far_threshold, mid..=2000.0),
                    );
                    actions.scene_dirty |= widget::row_hover(
                        ui,
                        "Hysteresis",
                        "Dead-zone around each LOD boundary. Prevents rapid geometry rebuilds \
                         when a plant sits right on a distance threshold.",
                        Slider::new(&mut settings.cull.lod_hysteresis, 0.0..=20.0),
                    );

                    for (label, value) in [
                        ("Leaf skip (near)", &mut settings.lod.leaf_skip_near),
                        ("Leaf skip (mid)", &mut settings.lod.leaf_skip_mid),
                        ("Leaf skip (far)", &mut settings.lod.leaf_skip_far),
                    ] {
                        actions.scene_dirty |= widget::row_hover(
                            ui,
                            label,
                            "Fraction of leaf quads randomly skipped at this LOD distance. 0 = \
                             off.",
                            percent_slider(value),
                        );
                    }
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
    let (response, painter) =
        ui.allocate_painter(egui::Vec2::new(width, height), egui::Sense::hover());
    painter.rect_filled(response.rect, 2.0, egui::Color32::from_black_alpha(120));
    if history.is_empty() {
        return;
    }

    let bar_w = response.rect.width() / history.len() as f32;

    for (i, &val) in history.iter().enumerate() {
        let h = (val / max_val).min(1.0) * response.rect.height();
        let x = response.rect.left() + i as f32 * bar_w;

        let colour = thresholds
            .iter()
            .find(|&&(t, _)| val > t)
            .map(|&(_, c)| c)
            .unwrap_or(default_colour);

        painter.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(x, response.rect.bottom() - h),
                egui::Vec2::new(bar_w.max(1.0), h),
            ),
            0.0,
            colour,
        );
    }

    for &(thresh, _) in thresholds {
        let y = response.rect.bottom() - (thresh / max_val).min(1.0) * response.rect.height();
        painter.line_segment(
            [
                egui::pos2(response.rect.left(), y),
                egui::pos2(response.rect.right(), y),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_gray(180)),
        );
    }
}

/// Per-stage rebuild timings
fn phase_breakdown_ui(ui: &mut egui::Ui, breakdown: &RebuildBreakdown) {
    let total = breakdown.total_ms();
    if total <= 0.0 {
        return;
    }

    Grid::new("rebuild_phase_grid")
        .num_columns(3)
        .spacing([6.0, 1.0])
        .show(ui, |ui| {
            for (name, ms) in breakdown.phases() {
                let share = ms / total;
                ui.label(egui::RichText::new(name).small());
                ui.label(egui::RichText::new(format!("{ms:.1} ms")).small());
                // A share bar makes the dominant stage readable at a glance.
                let (response, painter) =
                    ui.allocate_painter(egui::Vec2::new(60.0, 8.0), egui::Sense::hover());
                painter.rect_filled(response.rect, 1.0, egui::Color32::from_black_alpha(120));
                let filled = egui::Rect::from_min_size(
                    response.rect.min,
                    egui::Vec2::new(response.rect.width() * share, response.rect.height()),
                );
                painter.rect_filled(filled, 1.0, egui::Color32::from_rgb(120, 180, 240));
                ui.end_row();
            }
        });

    // Partial rebuilds only touch a few plants, the per-plant stages shrink with it but `combine` doesn't
    ui.label(
        egui::RichText::new(format!("{} plants rebuilt", breakdown.plants_built))
            .small()
            .weak(),
    );
}

fn debug_ui(ctx: &Context, debug: &DebugInfo, perf: &mut PerfLogger) {
    egui::Window::new("Debug")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::LEFT_TOP, [8.0, 8.0])
        .show(ctx, |ui| {
            // Frame timing
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

            // Geometry
            ui.separator();
            ui.label(format!("Triangles: {}", debug.mesh_index_count / 3));
            ui.label(format!("Line verts: {}", debug.scene.line_vertex_count));

            // Scene / LOD
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

            // Scene rebuilds
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
                phase_breakdown_ui(ui, &debug.scene.breakdown);
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

            // Camera
            ui.separator();
            let [x, y, z] = debug.camera_pos;
            ui.label(format!("Camera: ({x:.1}, {y:.1}, {z:.1})"));

            // Performance recording
            ui.separator();
            ui.strong("Performance Recording");
            if perf.is_recording() {
                let elapsed = perf.elapsed_secs();
                let s = &perf.summary;
                ui.colored_label(
                    egui::Color32::from_rgb(220, 60, 60),
                    format!("● REC  {:.1} s  |  {} frames", elapsed, s.frame_count),
                );
                ui.label(format!(
                    "Frame ms  min {:.1}  mean {:.1}  max {:.1}",
                    s.min_frame_ms,
                    s.mean_frame_ms(),
                    s.max_frame_ms,
                ));
                if s.rebuild_count > 0 {
                    ui.label(format!(
                        "Rebuilds  {}  (mean {:.1} ms)",
                        s.rebuild_count,
                        s.mean_rebuild_ms(),
                    ));
                    let mean = s.mean_breakdown();
                    let (name, share) = mean.dominant();
                    ui.label(
                        egui::RichText::new(format!("dominated by {name} ({:.0}%)", share * 100.0))
                            .small(),
                    );
                    phase_breakdown_ui(ui, &mean);
                }
                if let Some(f) = &perf.current_file {
                    ui.label(egui::RichText::new(f.as_str()).small().weak());
                }
                if ui
                    .button("■ Stop")
                    .on_hover_text("Flush and close the CSV file")
                    .clicked()
                {
                    perf.stop();
                }
            } else {
                // Show summary from last session if available.
                let s = &perf.summary;
                if s.frame_count > 0 {
                    ui.label(format!(
                        "Last: {} frames  min {:.1}  mean {:.1}  max {:.1} ms",
                        s.frame_count,
                        s.min_frame_ms,
                        s.mean_frame_ms(),
                        s.max_frame_ms,
                    ));
                    if s.rebuild_count > 0 {
                        ui.label(format!(
                            "Rebuilds  {}  (mean {:.1} ms)",
                            s.rebuild_count,
                            s.mean_rebuild_ms(),
                        ));
                    }
                    if let Some(f) = &perf.current_file {
                        ui.label(egui::RichText::new(f.as_str()).small().weak());
                    }
                }
                if ui
                    .button("▶ Start Recording")
                    .on_hover_text("Write per-frame CSV to the working directory")
                    .clicked()
                {
                    perf.start();
                }
            }
        });
}
