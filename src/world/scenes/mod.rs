use std::f32::consts::FRAC_PI_2;

use glam::Vec3;
use wgpu::{Buffer, Queue};

use crate::{
    settings::{PlantType, SceneData, SceneSettings},
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
        }
    }

    pub fn set_scene(
        &mut self,
        settings: &mut SceneSettings,
        buffers: &SceneBuffers,
        ground_color: Vec3,
    ) -> (Vec<(u32, u32)>, u32) {
        let scene = settings.active_mut();

        if !scene.dirty && !self.line_segments.is_empty() {
            return (self.line_segments.clone(), self.mesh_index_count);
        }
        scene.dirty = false;

        // Forest scene regenerates its plant list from the current seed
        if scene.name == "Forest" {
            scene.plants = generate_forest_plants(42);
        }
        rng::seed(42);

        let mut line_geos = Vec::new();
        let mut mesh_geos = Vec::new();

        let global_age = scene.global_age;
        let global_scale = scene.global_scale;

        let mut turtle = Turtle::new(Vec3::ZERO, Vec3::ZERO);

        for plant in &mut scene.plants {
            plant.plant.set_age(plant.base_age + global_age);

            let pos = Vec3::from(plant.position);
            let colour = plant.plant.colour();
            let scale = plant.scale * global_scale;

            turtle.reset(pos, colour);
            turtle.set_scale(scale);

            turtle.roll(FRAC_PI_2);
            turtle.turn(FRAC_PI_2);
            turtle.roll(plant.rotation.to_radians());
            turtle.do_actions(plant.plant.actions());
            line_geos.push(turtle.line_geometry());
            mesh_geos.push(turtle.mesh_geometry());
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
