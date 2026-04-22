use std::{
    f32::consts::{FRAC_PI_2, TAU},
    time::Instant,
    vec,
};

use glam::{Mat4, Vec3, vec3};
use wgpu::{Buffer, Queue};

use super::lod::{Frustum, LodTier, PlantCache, SceneStats, plant_lod_tier};
use crate::{
    settings::{CullSettings, EnvironmentSettings, LodSettings, SceneSettings, WorldDate},
    util::{
        rng,
        turtle::{
            LineGeometry, MeshGeometry, Turtle, combine_line_geometries, combine_mesh_geometries,
        },
    },
    world::{ecosystem::generate_ecosystem_plants, plants::PlantEnvironment},
};

/// Maximum number of vertices/indices per GPU buffer — must match the allocation in renderer.rs.
const MAX_VERTS: usize = 20_000_000;

/// Milliseconds since `start`, for the rebuild phase counters
fn elapsed_ms(start: Instant) -> f32 {
    start.elapsed().as_secs_f32() * 1000.0
}

/// Per-frame view and rendering parameters
pub struct FrameParams<'a> {
    pub camera_pos: Vec3,
    pub view_proj: Mat4,
    pub buffers: &'a SceneBuffers<'a>,
    pub ground_colour: Vec3,
    pub debug_mode: bool,
}

pub struct SceneOutput {
    pub line_segments: Vec<(u32, u32)>,
    pub debug_line_segments: Vec<(u32, u32)>,
    pub mesh_index_count: u32,
}

pub struct SceneBuffers<'a> {
    pub queue: &'a Queue,
    pub line_vertex: &'a Buffer,
    pub line_colour: &'a Buffer,
    pub line_index: &'a Buffer,
    pub mesh_vertex: &'a Buffer,
    pub mesh_normal: &'a Buffer,
    pub mesh_colour: &'a Buffer,
    pub mesh_uv: &'a Buffer,
    pub mesh_tangent: &'a Buffer,
    pub mesh_index: &'a Buffer,
}

impl SceneBuffers<'_> {
    /// Writes each `(buffer, bytes)` pair to the GPU from offset 0.
    fn upload(&self, writes: &[(&Buffer, &[u8])]) {
        for (buffer, bytes) in writes {
            self.queue.write_buffer(buffer, 0, bytes);
        }
    }
}

// ── Ground Plane ──

const GROUND_SIZE: f32 = 200.0;

fn ground_geometry(colour: Vec3) -> MeshGeometry {
    let e = GROUND_SIZE; // half-extent: ground spans [-e, e] on X and Z
    let vertices = vec![
        Vec3::new(-e, 0.0, -e),
        Vec3::new(e, 0.0, -e),
        Vec3::new(-e, 0.0, e),
        Vec3::new(e, 0.0, e),
    ];
    MeshGeometry {
        normals: vec![Vec3::Y; 4],
        colours: vec![colour; 4],
        indices: vec![0, 2, 1, 1, 2, 3],
        uvs: vec![[-1.0f32, -1.0]; 4],
        tangents: vec![Vec3::ZERO; 4],
        vertices,
    }
}

// ── LOD debug circles ──

/// Horizontal circle at ground level for LOD boundary visualisation.
fn lod_circle_geometry(centre: Vec3, radius: f32, colour: Vec3) -> LineGeometry {
    const SEGMENTS: usize = 64;

    let vertices: Vec<Vec3> = (0..=SEGMENTS)
        .map(|i| {
            let a = TAU * i as f32 / SEGMENTS as f32;
            vec3(
                centre.x + radius * a.cos(),
                0.05,
                centre.z + radius * a.sin(),
            )
        })
        .collect();

    LineGeometry {
        colours: vec![colour; SEGMENTS],
        indices: (0..SEGMENTS as u32).collect(),
        segments: vec![(0, SEGMENTS as u32)],
        vertices,
    }
}

// ── Turtle configuration ──

/// Per-frame turtle inputs
struct TurtleConfig<'a> {
    env: &'a EnvironmentSettings,
    global_scale: f32,
    date: WorldDate,
    wind_dir: Vec3,
}

impl<'a> TurtleConfig<'a> {
    fn from_env_scene(
        env: &'a EnvironmentSettings,
        scene_global_scale: f32,
        date: WorldDate,
    ) -> Self {
        let azimuth = env.wind_azimuth.to_radians();
        Self {
            env,
            global_scale: scene_global_scale,
            date,
            wind_dir: Vec3::new(azimuth.cos(), 0.0, azimuth.sin()),
        }
    }

    /// Configure the turtle for a specific plant position, colour, and scale
    fn apply(&self, turtle: &mut Turtle, pos: Vec3, colour: Vec3, plant_scale: f32, rotation: f32) {
        let env = self.env;
        let light_dir = (Vec3::from(env.light_position) - pos).normalize_or_zero();

        turtle.reset(pos, colour);
        turtle.set_scale(plant_scale * self.global_scale);
        turtle.set_tropism(light_dir, env.tropism_strength);
        turtle.set_gravitropism(env.gravitropism_strength);
        turtle.set_wind(self.wind_dir, env.wind_strength, env.wind_turbulence);
        turtle.set_wind_baked(env.wind_baked);
        turtle.set_taper(env.taper);
        turtle.set_proprioception(env.proprioception_gamma);
        turtle.set_variation(
            env.variation_noise_strength,
            env.variation_decay_rate,
            env.colour_variation,
        );
        turtle.set_space_pruning(env.space_pruning, env.occupancy_cell_size);
        turtle.roll(FRAC_PI_2);
        turtle.turn(FRAC_PI_2);
        turtle.roll(rotation.to_radians());
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
    debug_line_segments: Vec<(u32, u32)>,
    mesh_index_count: u32,
    cached_ground: Option<(Vec3, MeshGeometry)>,
    plant_caches: Vec<PlantCache>,
    last_debug_mode: bool,
    last_scene_gen: u64,
    last_env_gen: u64,
    last_eco_gen: u64,
    last_active_scene: usize,
    stats: SceneStats,
    lod_pressure: f32,
    last_pressure: f32,
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
            debug_line_segments: Vec::new(),
            mesh_index_count: 0,
            cached_ground: None,
            plant_caches: Vec::new(),
            last_debug_mode: false,
            last_scene_gen: u64::MAX,
            last_env_gen: u64::MAX,
            last_eco_gen: u64::MAX,
            last_active_scene: usize::MAX,
            stats: SceneStats::default(),
            lod_pressure: 1.0,
            last_pressure: 1.0,
        }
    }

    pub fn scene_stats(&self) -> SceneStats {
        self.stats
    }

    // ── Public entry point ──

    pub fn set_scene(
        &mut self,
        settings: &mut SceneSettings,
        env: &mut EnvironmentSettings,
        lod: &LodSettings,
        cull: &CullSettings,
        frame: &FrameParams,
    ) -> SceneOutput {
        let active_scene_idx = settings.active_scene;
        let scene = settings.active_mut();
        let frustum = Frustum::from_view_proj(frame.view_proj);

        let eco_changed = self.update_ecosystem(scene, active_scene_idx);

        let pressure_changed = (self.lod_pressure - self.last_pressure).abs() > 1e-4;
        let full_rebuild = eco_changed
            || scene.generation != self.last_scene_gen
            || env.generation != self.last_env_gen
            || self.plant_caches.len() != scene.plants.len()
            || pressure_changed;

        let effective = self.effective_lod(lod);
        let lod = &effective;

        let desired_tiers =
            self.compute_desired_tiers(scene, frame.camera_pos, &frustum, lod, cull, full_rebuild);

        let any_changed = full_rebuild
            || frame.debug_mode != self.last_debug_mode
            || desired_tiers
                .iter()
                .enumerate()
                .any(|(i, &t)| t != self.plant_caches[i].requested_tier);

        if !any_changed && !self.line_segments.is_empty() {
            return SceneOutput {
                line_segments: self.line_segments.clone(),
                debug_line_segments: self.debug_line_segments.clone(),
                mesh_index_count: self.mesh_index_count,
            };
        }

        let build_start = Instant::now();
        self.last_pressure = self.lod_pressure;

        let config = TurtleConfig::from_env_scene(env, scene.global_scale, scene.date);
        let scene_seed = scene.seed;
        self.build_plant_caches(
            scene,
            &config,
            &desired_tiers,
            lod,
            full_rebuild,
            scene_seed,
        );

        self.last_debug_mode = frame.debug_mode;
        self.last_scene_gen = scene.generation;
        self.last_env_gen = env.generation;

        let (overflow, fill) = self.assemble_and_upload(frame, lod);
        self.update_stats(build_start, full_rebuild, lod);

        if overflow {
            // Geometry exceeded GPU buffer, tighten culling for next frame
            self.lod_pressure = (self.lod_pressure * 0.75).max(0.1);
        } else if fill < 0.65 && self.lod_pressure < 1.0 {
            // Significant headroom, ease culling back toward nominal
            self.lod_pressure = (self.lod_pressure / 0.9).min(1.0);
        }

        SceneOutput {
            line_segments: self.line_segments.clone(),
            debug_line_segments: self.debug_line_segments.clone(),
            mesh_index_count: self.mesh_index_count,
        }
    }

    /// Returns LOD settings adjusted by the current pressure multiplier
    fn effective_lod(&self, lod: &LodSettings) -> LodSettings {
        let p = self.lod_pressure;
        LodSettings {
            near_threshold: lod.near_threshold * p,
            mid_threshold: lod.mid_threshold * p,
            far_threshold: lod.far_threshold * p,
            leaf_skip_near: 1.0 - (1.0 - lod.leaf_skip_near) * p,
            leaf_skip_mid: 1.0 - (1.0 - lod.leaf_skip_mid) * p,
            leaf_skip_far: 1.0 - (1.0 - lod.leaf_skip_far) * p,
        }
    }

    /// Regenerates plants if the ecosystem or active scene has changed
    fn update_ecosystem(
        &mut self,
        scene: &mut crate::settings::SceneData,
        active_scene_idx: usize,
    ) -> bool {
        if active_scene_idx != self.last_active_scene
            || scene.ecosystem.generation != self.last_eco_gen
        {
            scene.plants = generate_ecosystem_plants(&scene.ecosystem, scene.seed);
            self.last_eco_gen = scene.ecosystem.generation;
            self.last_active_scene = active_scene_idx;
            self.plant_caches.clear();
            true
        } else {
            false
        }
    }

    /// Returns the desired LOD tier for every plant in the scene
    fn compute_desired_tiers(
        &self,
        scene: &crate::settings::SceneData,
        camera_pos: Vec3,
        frustum: &Frustum,
        lod: &LodSettings,
        cull: &CullSettings,
        full_rebuild: bool,
    ) -> Vec<LodTier> {
        scene
            .plants
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let prev = if full_rebuild || self.plant_caches.is_empty() {
                    LodTier::Culled
                } else {
                    self.plant_caches[i].tier
                };
                plant_lod_tier(
                    p.position,
                    camera_pos,
                    prev,
                    frustum,
                    lod,
                    cull,
                    full_rebuild,
                )
            })
            .collect()
    }

    /// Runs the geometry build loop, updating `self.plant_caches` for every plant whose tier has changed (or all plants on a full rebuild)
    fn build_plant_caches(
        &mut self,
        scene: &mut crate::settings::SceneData,
        config: &TurtleConfig,
        desired_tiers: &[LodTier],
        lod: &LodSettings,
        full_rebuild: bool,
        scene_seed: u64,
    ) {
        if full_rebuild {
            self.plant_caches.clear();
        }

        let mut turtle = Turtle::new(Vec3::ZERO, Vec3::ZERO);
        turtle.clear_occupancy();

        for (i, plant) in scene.plants.iter_mut().enumerate() {
            if !full_rebuild && desired_tiers[i] == self.plant_caches[i].requested_tier {
                // No change in LOD tier
                continue;
            }

            // Cull Plant
            if desired_tiers[i] == LodTier::Culled {
                let cache = PlantCache::empty(LodTier::Culled);
                if full_rebuild {
                    self.plant_caches.push(cache);
                } else {
                    self.plant_caches[i] = cache;
                }
                continue;
            }

            // Age plant
            let age_years = (config.date.total_years() - plant.delay_years).max(0.0);
            let effective_iter = age_years / plant.plant.plant_type().years_per_iteration();
            plant
                .plant
                .set_iteration((effective_iter as u32).min(plant.plant.max_iterations()));

            let env = PlantEnvironment {
                season: config.date.season(),
            };

            rng::seed(scene_seed.wrapping_add((i as u64).wrapping_mul(6364136223846793005u64)));

            let (lod_segments, lod_min_radius) = desired_tiers[i].geometry_params();
            config.apply(
                &mut turtle,
                plant.position,
                plant.plant.colour(),
                plant.scale,
                plant.rotation,
            );
            // set_lod must come after config.apply — apply calls reset() which
            // restores default LOD parameters.
            turtle.set_lod(lod_segments, lod_min_radius);
            turtle.set_leaf_skip_prob(desired_tiers[i].leaf_skip(lod));
            turtle.do_actions(plant.plant.actions(&env));

            let cache = PlantCache {
                tier: desired_tiers[i],
                requested_tier: desired_tiers[i],
                line_geo: turtle.line_geometry(),
                mesh_geo: turtle.take_mesh_geometry(),
            };
            if full_rebuild {
                self.plant_caches.push(cache);
            } else {
                self.plant_caches[i] = cache;
            }
        }
    }

    /// Upload combined plant geometries, ground and debug circles to the GPU. Returns `(overflow, fill)` where `overflow` is true when any buffer would exceed `MAX_VERTS` and `fill` is the fraction of MAX_VERTS used by the largest buffer.
    fn assemble_and_upload(&mut self, frame: &FrameParams, lod: &LodSettings) -> (bool, f32) {
        let mut line_geo_refs: Vec<&LineGeometry> =
            self.plant_caches.iter().map(|c| &c.line_geo).collect();
        let mut mesh_geo_refs: Vec<&MeshGeometry> =
            self.plant_caches.iter().map(|c| &c.mesh_geo).collect();

        // Debug circles
        let debug_circles: [LineGeometry; 3];
        if frame.debug_mode {
            debug_circles = [
                lod_circle_geometry(frame.camera_pos, lod.near_threshold, vec3(0.2, 1.0, 0.3)),
                lod_circle_geometry(frame.camera_pos, lod.mid_threshold, vec3(1.0, 1.0, 0.2)),
                lod_circle_geometry(frame.camera_pos, lod.far_threshold, vec3(1.0, 0.3, 0.2)),
            ];
            line_geo_refs.extend(debug_circles.iter());
        }

        // Ground plane
        if self
            .cached_ground
            .as_ref()
            .is_none_or(|(c, _)| *c != frame.ground_colour)
        {
            self.cached_ground = Some((frame.ground_colour, ground_geometry(frame.ground_colour)));
        }
        let ground = &self.cached_ground.as_ref().unwrap().1;
        mesh_geo_refs.push(ground);

        let combined_lines = combine_line_geometries(&line_geo_refs);
        let combined_mesh = combine_mesh_geometries(&mesh_geo_refs);

        // Only write and update CPU state when geometry fits in the allocated buffers.
        // If it doesn't fit, the previous frame's buffer contents and draw counts remain.
        if !combined_lines.vertices.is_empty()
            && combined_lines.vertices.len() <= MAX_VERTS
            && combined_lines.indices.len() <= MAX_VERTS
        {
            let debug_start_seg_idx: usize = self
                .plant_caches
                .iter()
                .map(|c| c.line_geo.segments.len())
                .sum();
            let (plant_segs, debug_segs) = combined_lines
                .segments
                .split_at(debug_start_seg_idx.min(combined_lines.segments.len()));

            self.line_segments = plant_segs.to_vec();
            self.debug_line_segments = debug_segs.to_vec();

            frame.buffers.upload(&[
                (
                    frame.buffers.line_vertex,
                    bytemuck::cast_slice(&combined_lines.vertices),
                ),
                (
                    frame.buffers.line_colour,
                    bytemuck::cast_slice(&combined_lines.colours),
                ),
                (
                    frame.buffers.line_index,
                    bytemuck::cast_slice(&combined_lines.indices),
                ),
            ]);
        }

        if !combined_mesh.indices.is_empty()
            && combined_mesh.vertices.len() <= MAX_VERTS
            && combined_mesh.indices.len() <= MAX_VERTS
        {
            frame.buffers.upload(&[
                (
                    frame.buffers.mesh_vertex,
                    bytemuck::cast_slice(&combined_mesh.vertices),
                ),
                (
                    frame.buffers.mesh_normal,
                    bytemuck::cast_slice(&combined_mesh.normals),
                ),
                (
                    frame.buffers.mesh_colour,
                    bytemuck::cast_slice(&combined_mesh.colours),
                ),
                (
                    frame.buffers.mesh_uv,
                    bytemuck::cast_slice(&combined_mesh.uvs),
                ),
                (
                    frame.buffers.mesh_tangent,
                    bytemuck::cast_slice(&combined_mesh.tangents),
                ),
                (
                    frame.buffers.mesh_index,
                    bytemuck::cast_slice(&combined_mesh.indices),
                ),
            ]);

            self.mesh_index_count = combined_mesh.indices.len() as u32;
        }

        // Store line vertex count for stats (use combined_lines even if write was skipped —
        // the stat is informational and the previously-written count is preserved).
        self.stats.line_vertex_count = combined_lines.vertices.len() as u32;

        let fill = [
            combined_lines.vertices.len(),
            combined_lines.indices.len(),
            combined_mesh.vertices.len(),
            combined_mesh.indices.len(),
        ]
        .into_iter()
        .map(|n| n as f32 / MAX_VERTS as f32)
        .fold(0.0f32, f32::max);

        (fill > 1.0, fill)
    }

    /// Updates `self.stats` from the current plant cache state.
    fn update_stats(&mut self, build_start: Instant, full_rebuild: bool, lod: &LodSettings) {
        let mut lod_tier_counts = [0usize; 4];
        let mut culled_count = 0usize;

        for cache in &self.plant_caches {
            if cache.tier == LodTier::Culled {
                culled_count += 1;
            } else {
                lod_tier_counts[cache.tier as usize] += 1;
            }
        }

        self.stats = SceneStats {
            plant_count: self.plant_caches.len(),
            culled_count,
            lod_tier_counts,
            last_rebuild_ms: elapsed_ms(build_start),
            last_rebuild_full: full_rebuild,
            line_vertex_count: self.stats.line_vertex_count,
            lod_pressure: self.lod_pressure,
            lod_thresholds: [lod.near_threshold, lod.mid_threshold, lod.far_threshold],
        };
    }
}
