use std::{
    f32::consts::{FRAC_PI_2, TAU},
    time::Instant,
};

use glam::{Mat4, Vec3, vec3};
use wgpu::{Buffer, Queue};

use super::lod::{LodTier, PlantCache, SceneStats, plant_lod_tier};
use crate::{
    settings::{CullSettings, EnvironmentSettings, LodSettings, SceneSettings, WorldDate},
    util::{
        rng,
        turtle::{
            LineGeometry, MeshGeometry, Turtle, combine_line_geometries, combine_mesh_geometries,
        },
    },
    world::plants::PlantEnvironment,
};

/// Maximum number of vertices/indices per GPU buffer — must match the allocation in renderer.rs.
const MAX_VERTS: usize = 20_000_000;

// ── Public types ──

/// Per-frame view and rendering parameters, passed as a bundle to keep `set_scene`
/// within clippy's argument-count limit.
pub struct FrameParams<'a> {
    pub camera_pos: Vec3,
    pub view_proj: Mat4,
    pub buffers: &'a SceneBuffers<'a>,
    pub ground_colour: Vec3,
    pub debug_mode: bool,
}

/// Output of `set_scene`: the assembled line and mesh draw state for the current frame.
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

// ── Ground Plane ──

const GROUND_SIZE: f32 = 200.0;

fn ground_geometry(colour: Vec3) -> MeshGeometry {
    let e = GROUND_SIZE; // half-extent: ground spans [-e, e] on X and Z
    let normal = Vec3::Y;
    let vertices = vec![
        Vec3::new(-e, 0.0, -e),
        Vec3::new(e, 0.0, -e),
        Vec3::new(-e, 0.0, e),
        Vec3::new(e, 0.0, e),
    ];
    MeshGeometry {
        normals: vec![normal; 4],
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
    let cx = centre.x;
    let cz = centre.z;
    let vertices: Vec<Vec3> = (0..=SEGMENTS)
        .map(|i| {
            let a = TAU * i as f32 / SEGMENTS as f32;
            vec3(cx + radius * a.cos(), 0.05, cz + radius * a.sin())
        })
        .collect();
    let n = vertices.len();
    LineGeometry {
        colours: vec![colour; n],
        indices: (0..n as u32).collect(),
        segments: vec![(0, n as u32)],
        vertices,
    }
}

// ── Turtle configuration ──

/// All per-frame turtle parameters derived from environment and scene settings.
/// Computed once before the plant loop and reused for every plant.
struct TurtleConfig {
    light_pos: Vec3,
    global_scale: f32,
    date: WorldDate,
    tropism_strength: f32,
    gravitropism_strength: f32,
    wind_dir: Vec3,
    wind_strength: f32,
    wind_turbulence: f32,
    wind_baked: bool,
    taper: f32,
    proprioception_gamma: f32,
    variation_noise_strength: f32,
    variation_decay_rate: f32,
    colour_variation: f32,
}

impl TurtleConfig {
    fn from_env_scene(env: &EnvironmentSettings, scene_global_scale: f32, date: WorldDate) -> Self {
        let wind_azimuth = env.wind_azimuth.to_radians();
        Self {
            light_pos: Vec3::from(env.light_position),
            global_scale: scene_global_scale,
            date,
            tropism_strength: env.tropism_strength,
            gravitropism_strength: env.gravitropism_strength,
            wind_dir: Vec3::new(wind_azimuth.cos(), 0.0, wind_azimuth.sin()),
            wind_strength: env.wind_strength,
            wind_turbulence: env.wind_turbulence,
            wind_baked: env.wind_baked,
            taper: env.taper,
            proprioception_gamma: env.proprioception_gamma,
            variation_noise_strength: env.variation_noise_strength,
            variation_decay_rate: env.variation_decay_rate,
            colour_variation: env.colour_variation,
        }
    }

    /// Configure the turtle for a specific plant position, colour, and scale.
    fn apply(&self, turtle: &mut Turtle, pos: Vec3, colour: Vec3, plant_scale: f32, rotation: f32) {
        let light_dir = (self.light_pos - pos).normalize_or_zero();
        let scale = plant_scale * self.global_scale;

        turtle.reset(pos, colour);
        turtle.set_scale(scale);
        turtle.set_tropism(light_dir, self.tropism_strength);
        turtle.set_gravitropism(self.gravitropism_strength);
        turtle.set_wind(self.wind_dir, self.wind_strength, self.wind_turbulence);
        turtle.set_wind_baked(self.wind_baked);
        turtle.set_taper(self.taper);
        turtle.set_proprioception(self.proprioception_gamma);
        turtle.set_variation(
            self.variation_noise_strength,
            self.variation_decay_rate,
            self.colour_variation,
        );
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
    /// Per-plant cached geometry — keyed by LOD tier; empty when LOD_CULLED.
    plant_caches: Vec<PlantCache>,
    last_debug_mode: bool,
    last_scene_gen: u64,
    last_env_gen: u64,
    stats: SceneStats,
    /// Pressure multiplier applied to LOD thresholds and leaf-skip when geometry
    /// exceeds MAX_VERTS. 1.0 = nominal; < 1.0 = tighter culling.
    lod_pressure: f32,
    /// Pressure used for the most recent geometry build (compared each frame to detect change).
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
            // u64::MAX / usize::MAX ensures the first frame always triggers a rebuild.
            last_scene_gen: u64::MAX,
            last_env_gen: u64::MAX,
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
        let scene = settings.active_mut();

        let pressure_changed = (self.lod_pressure - self.last_pressure).abs() > 1e-4;
        let full_rebuild = scene.generation != self.last_scene_gen
            || env.generation != self.last_env_gen
            || self.plant_caches.len() != scene.plants.len()
            || pressure_changed;

        let effective = self.effective_lod(lod);
        let lod = &effective;

        let desired_tiers =
            self.compute_desired_tiers(scene, frame.camera_pos, lod, cull, full_rebuild);

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
            // Geometry exceeded GPU buffer — tighten culling for next frame.
            self.lod_pressure = (self.lod_pressure * 0.75).max(0.1);
        } else if fill < 0.65 && self.lod_pressure < 1.0 {
            // Significant headroom — ease culling back toward nominal.
            self.lod_pressure = (self.lod_pressure / 0.9).min(1.0);
        }

        SceneOutput {
            line_segments: self.line_segments.clone(),
            debug_line_segments: self.debug_line_segments.clone(),
            mesh_index_count: self.mesh_index_count,
        }
    }

    // ── Private helpers ──

    /// Returns LOD settings adjusted by the current pressure multiplier.
    /// Thresholds are scaled down (fewer plants get high-detail meshes) and
    /// leaf-skip probabilities are scaled up (fewer leaf quads).
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

    /// Returns the desired LOD tier for every plant in the scene.
    fn compute_desired_tiers(
        &self,
        scene: &crate::settings::SceneData,
        camera_pos: Vec3,
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
                plant_lod_tier(p.position, camera_pos, prev, lod, cull, full_rebuild)
            })
            .collect()
    }

    /// Runs the geometry build loop, updating `self.plant_caches` for every plant
    /// whose tier has changed (or all plants on a full rebuild).
    fn build_plant_caches(
        &mut self,
        scene: &mut crate::settings::SceneData,
        config: &TurtleConfig,
        desired_tiers: &[LodTier],
        lod: &LodSettings,
        full_rebuild: bool,
        scene_seed: u64,
    ) {
        let n = scene.plants.len();
        if full_rebuild {
            self.plant_caches.clear();
            self.plant_caches.reserve(n);
        }

        let mut turtle = Turtle::new(Vec3::ZERO, Vec3::ZERO);

        for (i, plant) in scene.plants.iter_mut().enumerate() {
            let tier = desired_tiers[i];

            if !full_rebuild && tier == self.plant_caches[i].requested_tier {
                continue;
            }

            if tier == LodTier::Culled {
                let cache = PlantCache::empty(LodTier::Culled);
                if full_rebuild {
                    self.plant_caches.push(cache);
                } else {
                    self.plant_caches[i] = cache;
                }
                continue;
            }

            let pt = plant.plant.plant_type();
            let age_years = (config.date.total_years() - plant.delay_years).max(0.0);
            let effective_iter = (age_years / pt.years_per_iteration()) as u32;
            let effective_iter = effective_iter.min(plant.plant.max_iterations());
            plant.plant.set_iteration(effective_iter);
            let env = PlantEnvironment {
                season: config.date.season(),
            };

            // Deterministic per-plant seed — geometry is independent of build order,
            // enabling correct partial rebuilds.
            rng::seed(scene_seed.wrapping_add(i as u64 * 6364136223846793005u64));

            let (lod_segments, lod_min_radius) = tier.geometry_params();
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
            turtle.set_leaf_skip_prob(tier.leaf_skip(lod));
            turtle.do_actions(plant.plant.actions(&env));

            let cache = PlantCache {
                tier,
                requested_tier: tier,
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

    /// Combines all cached plant geometries with the ground and optional debug circles,
    /// uploads to the GPU, and updates `self.line_segments` / `self.mesh_index_count`.
    /// Returns `(overflow, fill)` where `overflow` is true when any buffer would exceed
    /// `MAX_VERTS` and `fill` is the fraction of MAX_VERTS used by the largest buffer.
    fn assemble_and_upload(&mut self, frame: &FrameParams, lod: &LodSettings) -> (bool, f32) {
        let camera_pos = frame.camera_pos;
        let ground_colour = frame.ground_colour;
        let debug_mode = frame.debug_mode;
        let buffers = frame.buffers;
        let debug_start_seg_idx: usize = self
            .plant_caches
            .iter()
            .map(|c| c.line_geo.segments.len())
            .sum();

        // Debug circles are only materialised when needed — avoids any allocation on normal frames.
        let debug_circles: [LineGeometry; 3];
        let mut line_geo_refs: Vec<&LineGeometry> =
            self.plant_caches.iter().map(|c| &c.line_geo).collect();
        if debug_mode {
            debug_circles = [
                lod_circle_geometry(camera_pos, lod.near_threshold, vec3(0.2, 1.0, 0.3)),
                lod_circle_geometry(camera_pos, lod.mid_threshold, vec3(1.0, 1.0, 0.2)),
                lod_circle_geometry(camera_pos, lod.far_threshold, vec3(1.0, 0.3, 0.2)),
            ];
            line_geo_refs.extend(debug_circles.iter());
        }

        // Ground plane — cached so we don't re-create it every frame.
        if self
            .cached_ground
            .as_ref()
            .is_none_or(|(c, _)| *c != ground_colour)
        {
            self.cached_ground = Some((ground_colour, ground_geometry(ground_colour)));
        }
        let ground = &self.cached_ground.as_ref().unwrap().1;

        let mut mesh_geo_refs: Vec<&MeshGeometry> =
            self.plant_caches.iter().map(|c| &c.mesh_geo).collect();
        mesh_geo_refs.push(ground);

        let combined_lines = combine_line_geometries(&line_geo_refs);
        let combined_mesh = combine_mesh_geometries(&mesh_geo_refs);

        // Only write and update CPU state when geometry fits in the allocated buffers.
        // If it doesn't fit, the previous frame's buffer contents and draw counts remain.
        if !combined_lines.vertices.is_empty()
            && combined_lines.vertices.len() <= MAX_VERTS
            && combined_lines.indices.len() <= MAX_VERTS
        {
            let (plant_segs, debug_segs) = combined_lines
                .segments
                .split_at(debug_start_seg_idx.min(combined_lines.segments.len()));
            self.line_segments = plant_segs.to_vec();
            self.debug_line_segments = debug_segs.to_vec();

            buffers.queue.write_buffer(
                buffers.line_vertex,
                0,
                bytemuck::cast_slice(&combined_lines.vertices),
            );
            buffers.queue.write_buffer(
                buffers.line_colour,
                0,
                bytemuck::cast_slice(&combined_lines.colours),
            );
            buffers.queue.write_buffer(
                buffers.line_index,
                0,
                bytemuck::cast_slice(&combined_lines.indices),
            );
        }

        if !combined_mesh.indices.is_empty()
            && combined_mesh.vertices.len() <= MAX_VERTS
            && combined_mesh.indices.len() <= MAX_VERTS
        {
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
                buffers.mesh_colour,
                0,
                bytemuck::cast_slice(&combined_mesh.colours),
            );
            buffers.queue.write_buffer(
                buffers.mesh_uv,
                0,
                bytemuck::cast_slice(&combined_mesh.uvs),
            );
            buffers.queue.write_buffer(
                buffers.mesh_tangent,
                0,
                bytemuck::cast_slice(&combined_mesh.tangents),
            );
            buffers.queue.write_buffer(
                buffers.mesh_index,
                0,
                bytemuck::cast_slice(&combined_mesh.indices),
            );
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
            last_rebuild_ms: build_start.elapsed().as_secs_f32() * 1000.0,
            last_rebuild_full: full_rebuild,
            line_vertex_count: self.stats.line_vertex_count,
            lod_pressure: self.lod_pressure,
            lod_thresholds: [lod.near_threshold, lod.mid_threshold, lod.far_threshold],
        };
    }
}
