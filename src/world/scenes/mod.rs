use std::f32::consts::{FRAC_PI_2, TAU};

use glam::{Vec3, vec3};
use wgpu::{Buffer, Queue};

use crate::{
    settings::{
        EnvironmentSettings, LodSettings, PlantType, SceneData, SceneSettings, season_growth_factor,
    },
    util::{
        rng,
        turtle::{MeshGeometry, Turtle, combine_line_geometries, combine_mesh_geometries},
    },
    world::plants::PlantInstance,
};

// ── Hardcoded Scenes ──

pub fn hardcoded_scenes() -> Vec<SceneData> {
    vec![
        garden_scene(),
        meadow_scene(),
        botanical_scene(),
        forest_scene(),
        big_scene(),
    ]
}

fn big_scene() -> SceneData {
    let mut plants: Vec<PlantInstance> = Vec::new();
    let spacing = 5.0;

    rng::seed(0);

    for x in -25..25 {
        for z in -25..25 {
            let x = (x as f32 * spacing) + rng::random_range(-spacing / 2.0, spacing / 2.0);
            let z = (z as f32 * spacing) + rng::random_range(-spacing / 2.0, spacing / 2.0);

            plants.push(
                PlantInstance::new(PlantType::Wildflower, 10).with_transform([x, 0.0, z], 5.0, 0.0),
            );
        }
    }

    SceneData {
        name: "big".to_string(),
        plants,
        global_scale: 1.0,
        global_age: 0,
        dirty: true,
    }
}

fn garden_scene() -> SceneData {
    SceneData {
        name: "Garden".to_string(),
        plants: vec![
            PlantInstance::new(PlantType::Tree, 5).with_transform([0.0, 0.0, 0.0], 10.0, 0.0),
            PlantInstance::new(PlantType::Bush, 4).with_transform([15.0, 0.0, 10.0], 8.0, 45.0),
            PlantInstance::new(PlantType::Fern, 4).with_transform([-12.0, 0.0, 5.0], 6.0, 120.0),
            PlantInstance::new(PlantType::Mint, 4).with_transform([8.0, 0.0, -10.0], 7.0, 200.0),
            PlantInstance::new(PlantType::Tree, 4).with_transform([-8.0, 0.0, -15.0], 12.0, 270.0),
            PlantInstance::new(PlantType::Wildflower, 5).with_transform(
                [5.0, 0.0, -5.0],
                5.0,
                90.0,
            ),
        ],
        global_scale: 1.0,
        global_age: 0,
        dirty: true,
    }
}

fn meadow_scene() -> SceneData {
    SceneData {
        name: "Meadow".to_string(),
        plants: vec![
            PlantInstance::new(PlantType::Wildflower, 5).with_transform([0.0, 0.0, 0.0], 5.0, 0.0),
            PlantInstance::new(PlantType::Wildflower, 4).with_transform(
                [-8.0, 0.0, 6.0],
                6.0,
                45.0,
            ),
            PlantInstance::new(PlantType::Wildflower, 6).with_transform(
                [10.0, 0.0, -4.0],
                4.0,
                90.0,
            ),
            PlantInstance::new(PlantType::Lychnis, 4).with_transform(
                [-5.0, 0.0, -10.0],
                8.0,
                135.0,
            ),
            PlantInstance::new(PlantType::Lychnis, 3).with_transform([7.0, 0.0, 8.0], 7.0, 180.0),
            PlantInstance::new(PlantType::Carrot, 4).with_transform([-12.0, 0.0, -3.0], 6.0, 225.0),
            PlantInstance::new(PlantType::Carrot, 3).with_transform([3.0, 0.0, 12.0], 7.0, 270.0),
            PlantInstance::new(PlantType::Capsella, 4).with_transform([15.0, 0.0, 5.0], 5.0, 315.0),
        ],
        global_scale: 1.0,
        global_age: 0,
        dirty: true,
    }
}

fn botanical_scene() -> SceneData {
    SceneData {
        name: "Botanical".to_string(),
        plants: vec![
            PlantInstance::new(PlantType::Capsella, 5).with_transform([-20.0, 0.0, 0.0], 8.0, 0.0),
            PlantInstance::new(PlantType::Mint, 5).with_transform([-10.0, 0.0, 0.0], 7.0, 30.0),
            PlantInstance::new(PlantType::Lychnis, 4).with_transform([0.0, 0.0, 0.0], 9.0, 60.0),
            PlantInstance::new(PlantType::Mycelis, 5).with_transform([10.0, 0.0, 0.0], 8.0, 90.0),
            PlantInstance::new(PlantType::Carrot, 4).with_transform([20.0, 0.0, 0.0], 7.0, 120.0),
            PlantInstance::new(PlantType::Fern, 5).with_transform([-5.0, 0.0, -15.0], 8.0, 150.0),
            PlantInstance::new(PlantType::Bush, 4).with_transform([5.0, 0.0, -15.0], 7.0, 180.0),
        ],
        global_scale: 1.0,
        global_age: 0,
        dirty: true,
    }
}

fn forest_scene() -> SceneData {
    SceneData {
        name: "Forest".to_string(),
        plants: generate_forest_plants(0),
        global_scale: 1.0,
        global_age: 0,
        dirty: true,
    }
}

/// Generate forest plants from a seed so the layout changes with env.seed.
fn generate_forest_plants(seed: u64) -> Vec<PlantInstance> {
    rng::seed(seed);
    let types = [
        PlantType::Tree,
        PlantType::Tree,
        PlantType::Fern,
        PlantType::Bush,
    ];
    (0..12)
        .map(|_| {
            let x = rng::random_range(-40.0, 40.0);
            let z = rng::random_range(-40.0, 40.0);
            let age = 3 + (rng::random_range(0.0, 3.5) as u32);
            let scale = rng::random_range(8.0, 16.0);
            let rotation = rng::random_range(0.0, 360.0);
            let idx = (rng::random_range(0.0, types.len() as f32) as usize).min(types.len() - 1);
            PlantInstance::new(types[idx], age).with_transform([x, 0.0, z], scale, rotation)
        })
        .collect()
}

// ── Scene Buffers ──

pub struct SceneBuffers<'a> {
    pub queue: &'a Queue,
    pub line_vertex: &'a Buffer,
    pub line_color: &'a Buffer,
    pub line_index: &'a Buffer,
    pub mesh_vertex: &'a Buffer,
    pub mesh_normal: &'a Buffer,
    pub mesh_color: &'a Buffer,
    pub mesh_index: &'a Buffer,
}

// ── Ground Plane ──

const GROUND_SIZE: f32 = 200.0;

fn ground_geometry(color: Vec3) -> MeshGeometry {
    let y = 0.0;
    let half = GROUND_SIZE;
    let normal = Vec3::Y;

    let vertices = vec![
        Vec3::new(-half, y, -half),
        Vec3::new(half, y, -half),
        Vec3::new(-half, y, half),
        Vec3::new(half, y, half),
    ];
    let normals = vec![normal; 4];
    let colors = vec![color; 4];
    let indices = vec![0, 2, 1, 1, 2, 3];

    MeshGeometry {
        vertices,
        normals,
        colors,
        indices,
    }
}

// ── LOD helpers ──

fn lod_tier(dist: f32, lod: &LodSettings) -> u8 {
    if dist < lod.near_threshold {
        0
    } else if dist < lod.mid_threshold {
        1
    } else if dist < lod.far_threshold {
        2
    } else {
        3
    }
}

fn lod_params(tier: u8) -> (usize, f32) {
    match tier {
        0 => (8, 0.0),
        1 => (5, 0.0),
        2 => (3, 0.005),
        _ => (0, 1000.0), // skip mesh entirely for very distant plants
    }
}

// ── Debug Helpers ──

/// Horizontal circle at ground level for LOD boundary visualisation.
fn lod_circle_geometry(
    center: Vec3,
    radius: f32,
    color: Vec3,
) -> crate::util::turtle::LineGeometry {
    const SEGMENTS: usize = 64;
    let cx = center.x;
    let cz = center.z;
    let vertices: Vec<Vec3> = (0..=SEGMENTS)
        .map(|i| {
            let a = TAU * i as f32 / SEGMENTS as f32;
            vec3(cx + radius * a.cos(), 0.05, cz + radius * a.sin())
        })
        .collect();
    let n = vertices.len();
    crate::util::turtle::LineGeometry {
        colors: vec![color; n],
        indices: (0..n as u32).collect(),
        segments: vec![(0, n as u32)],
        vertices,
    }
}

// ── Scene Controller ──

impl std::fmt::Debug for SceneController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SceneController").finish()
    }
}

pub struct SceneController {
    line_segments: Vec<(u32, u32)>,
    mesh_index_count: u32,
    cached_ground: Option<(Vec3, MeshGeometry)>,
    last_lod_tiers: Vec<u8>,
    last_debug_camera_pos: Vec3,
}

impl Default for SceneController {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneController {
    pub fn new() -> Self {
        Self {
            line_segments: Vec::new(),
            mesh_index_count: 0,
            cached_ground: None,
            last_lod_tiers: Vec::new(),
            last_debug_camera_pos: Vec3::ZERO,
        }
    }

    pub fn set_scene(
        &mut self,
        settings: &mut SceneSettings,
        env: &mut EnvironmentSettings,
        lod: &LodSettings,
        camera_pos: Vec3,
        buffers: &SceneBuffers,
        ground_color: Vec3,
        debug_mode: bool,
    ) -> (Vec<(u32, u32)>, u32) {
        let scene = settings.active_mut();

        // Compute per-plant LOD tiers and trigger rebuild if any tier changed
        let current_tiers: Vec<u8> = scene
            .plants
            .iter()
            .map(|p| lod_tier((Vec3::from(p.position) - camera_pos).length(), lod))
            .collect();

        if current_tiers != self.last_lod_tiers {
            scene.dirty = true;
        }

        // In debug mode, rebuild whenever the camera moves so LOD circles track it.
        if debug_mode && (camera_pos - self.last_debug_camera_pos).length_squared() > 0.25 {
            scene.dirty = true;
        }

        if !scene.dirty && !env.dirty && !self.line_segments.is_empty() {
            return (self.line_segments.clone(), self.mesh_index_count);
        }
        scene.dirty = false;
        env.dirty = false;

        // Forest scene regenerates its plant list from the current seed
        if scene.name == "Forest" {
            scene.plants = generate_forest_plants(env.seed);
        }

        rng::seed(env.seed);

        let mut line_geos = Vec::new();
        let mut mesh_geos = Vec::new();

        let global_age = scene.global_age;
        let global_scale = scene.global_scale;
        let light_pos = Vec3::from(env.light_position);
        let tropism_strength = env.tropism_strength;
        let gravitropism_strength = env.gravitropism_strength;
        let wind_azimuth = env.wind_azimuth.to_radians();
        let wind_dir = Vec3::new(wind_azimuth.cos(), 0.0, wind_azimuth.sin());
        let wind_strength = env.wind_strength;
        let wind_turbulence = env.wind_turbulence;
        let taper = env.taper;
        let growth_factor = season_growth_factor(env.season);

        let mut turtle = Turtle::new(Vec3::ZERO, Vec3::ZERO);
        let mut accumulated_indices: u32 = 6; // ground plane is always 6 indices

        for plant in &mut scene.plants {
            let raw_age = plant.base_age + global_age;
            let effective_age =
                ((raw_age as f32 * growth_factor) as u32).min(plant.plant.max_age());
            plant.plant.set_age(effective_age);

            let pos = Vec3::from(plant.position);
            let colour = plant.plant.colour();
            let scale = plant.scale * global_scale;
            let light_dir = (light_pos - pos).normalize_or_zero();
            let dist = (pos - camera_pos).length();
            let base_tier = lod_tier(dist, lod);

            // LOD budget escalation: if adding this plant would exceed max_indices,
            // try progressively coarser tiers until it fits or tier 3 (skip mesh) is reached.
            let mut tier = base_tier;
            loop {
                let (lod_segments, lod_min_radius) = lod_params(tier);

                turtle.reset(pos, colour);
                turtle.set_scale(scale);
                turtle.set_lod(lod_segments, lod_min_radius);
                turtle.set_tropism(light_dir, tropism_strength);
                turtle.set_gravitropism(gravitropism_strength);
                turtle.set_wind(wind_dir, wind_strength, wind_turbulence);
                turtle.set_taper(taper);
                turtle.roll(FRAC_PI_2);
                turtle.turn(FRAC_PI_2);
                turtle.roll(plant.rotation.to_radians());
                turtle.do_actions(plant.plant.actions());

                let plant_indices = turtle.mesh_index_count();
                let fits = lod.max_indices == 0
                    || accumulated_indices + plant_indices <= lod.max_indices
                    || tier >= 3;

                if fits {
                    accumulated_indices += plant_indices;
                    break;
                }
                tier += 1;
            }

            line_geos.push(turtle.line_geometry());
            mesh_geos.push(turtle.mesh_geometry());
        }

        self.last_lod_tiers = current_tiers;

        // Debug: draw LOD threshold circles centred on the camera
        if debug_mode {
            self.last_debug_camera_pos = camera_pos;
            line_geos.push(lod_circle_geometry(
                camera_pos,
                lod.near_threshold,
                vec3(0.2, 1.0, 0.3),
            ));
            line_geos.push(lod_circle_geometry(
                camera_pos,
                lod.mid_threshold,
                vec3(1.0, 1.0, 0.2),
            ));
            line_geos.push(lod_circle_geometry(
                camera_pos,
                lod.far_threshold,
                vec3(1.0, 0.3, 0.2),
            ));
        }

        // Add ground plane (cached by colour)
        let ground = match &self.cached_ground {
            Some((c, geo)) if *c == ground_color => geo,
            _ => {
                self.cached_ground = Some((ground_color, ground_geometry(ground_color)));
                &self.cached_ground.as_ref().unwrap().1
            }
        };
        mesh_geos.push(ground.clone());

        let combined_lines = combine_line_geometries(&line_geos);
        let combined_mesh = combine_mesh_geometries(&mesh_geos);

        if !combined_lines.vertices.is_empty() {
            buffers.queue.write_buffer(
                buffers.line_vertex,
                0,
                bytemuck::cast_slice(&combined_lines.vertices),
            );
            buffers.queue.write_buffer(
                buffers.line_color,
                0,
                bytemuck::cast_slice(&combined_lines.colors),
            );
            buffers.queue.write_buffer(
                buffers.line_index,
                0,
                bytemuck::cast_slice(&combined_lines.indices),
            );
        }
        self.line_segments = combined_lines.segments;

        if !combined_mesh.indices.is_empty() {
            buffers.queue.write_buffer(
                buffers.mesh_vertex,
                0,
                bytemuck::cast_slice(&combined_mesh.vertices),
            );
            buffers.queue.write_buffer(
                buffers.mesh_normal,
                0,
                bytemuck::cast_slice(&combined_mesh.normals),
            );
            buffers.queue.write_buffer(
                buffers.mesh_color,
                0,
                bytemuck::cast_slice(&combined_mesh.colors),
            );
            buffers.queue.write_buffer(
                buffers.mesh_index,
                0,
                bytemuck::cast_slice(&combined_mesh.indices),
            );
        }
        self.mesh_index_count = combined_mesh.indices.len() as u32;

        (self.line_segments.clone(), self.mesh_index_count)
    }
}
