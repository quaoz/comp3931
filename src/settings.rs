use std::{collections::HashSet, f32::consts::TAU, fmt};

use glam::{Vec3, vec3};

use crate::world::plants::PlantInstance;

// ── Plant Types ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlantType {
    Tree,
    Bush,
    Fern,
    Wildflower,
    Capsella,
    Mint,
    Lychnis,
    Mycelis,
    Carrot,
}

impl PlantType {
    pub const ALL: [PlantType; 9] = [
        PlantType::Tree,
        PlantType::Bush,
        PlantType::Fern,
        PlantType::Wildflower,
        PlantType::Capsella,
        PlantType::Mint,
        PlantType::Lychnis,
        PlantType::Mycelis,
        PlantType::Carrot,
    ];

    /// Intrinsic shade tolerance [0, 1]. Higher = survives better under canopy competition.
    pub fn shade_tolerance(self) -> f32 {
        match self {
            PlantType::Tree => 0.75,
            PlantType::Bush => 0.50,
            PlantType::Fern => 0.90,
            PlantType::Wildflower => 0.20,
            PlantType::Capsella => 0.10,
            PlantType::Mint => 0.45,
            PlantType::Lychnis => 0.30,
            PlantType::Mycelis => 0.65,
            PlantType::Carrot => 0.25,
        }
    }

    /// Intrinsic growth rate (radius units per succession step).
    pub fn growth_rate(self) -> f32 {
        match self {
            PlantType::Tree => 0.04,
            PlantType::Bush => 0.08,
            PlantType::Fern => 0.10,
            PlantType::Wildflower => 0.18,
            PlantType::Capsella => 0.25,
            PlantType::Mint => 0.14,
            PlantType::Lychnis => 0.16,
            PlantType::Mycelis => 0.12,
            PlantType::Carrot => 0.17,
        }
    }

    /// Real-world years per L-system iteration for date-based age computation.
    /// Slow-growing species (trees) require many years per step;
    /// annuals complete their lifecycle in a fraction of a year.
    pub fn years_per_iteration(self) -> f32 {
        match self {
            PlantType::Tree => 5.0,        // 50 years to mature at max_age 10
            PlantType::Bush => 2.0,        // 12 years to mature at max_age 6
            PlantType::Fern => 0.1,        // ~2.5 years to mature at max_iterations 25
            PlantType::Wildflower => 0.08, // ~7 months to mature — annual
            PlantType::Capsella => 0.10,   // ~2 years to mature — biennial
            PlantType::Mint => 0.20,       // ~4 years to mature — perennial herb
            PlantType::Lychnis => 0.15,    // ~2 years to mature — biennial
            PlantType::Mycelis => 0.02,    // ~2 years to mature — annual/biennial
            PlantType::Carrot => 0.15,     // ~2 years to mature — biennial
        }
    }
}

impl fmt::Display for PlantType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// ── World Date ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WorldDate {
    pub year: u32,
    pub week: u32,
}

impl WorldDate {
    /// Season: 0 = spring, 0.25 = summer, 0.5 = autumn, 0.75 = winter
    pub fn season(self) -> f32 {
        self.week as f32 / 52.0
    }

    /// Total elapsed time in years
    pub fn total_years(self) -> f32 {
        self.year as f32 + self.week as f32 / 52.0
    }

    /// Advance by `weeks`, rolling over into the next year as needed
    pub fn advance_weeks(self, weeks: i32) -> Self {
        let total = self.year as i64 * 52 + self.week as i64 + weeks as i64;
        let total = total.max(0) as u64;
        Self {
            year: (total / 52) as u32,
            week: (total % 52) as u32,
        }
    }
}

// ── Scene Data ──

#[derive(Debug, Clone)]
pub struct SceneData {
    pub name: String,
    pub plants: Vec<PlantInstance>,
    pub global_scale: f32,
    pub date: WorldDate,
    pub seed: u64,
    pub ecosystem: EcosystemSettings,
    pub generation: u64,
}

impl SceneData {
    pub fn mark_dirty(&mut self) {
        self.generation += 1;
    }
}

// ── Scene Settings ──

#[derive(Debug, Clone)]
pub struct SceneSettings {
    pub scenes: Vec<SceneData>,
    pub active_scene: usize,
    pub hardcoded_names: HashSet<String>,
    /// Automatically advance the scene date each update tick
    pub auto_progress: bool,
    /// Date steps advanced per second when `auto_progress` is enabled
    pub progress_rate: f32,
    /// Weeks per step — also used by the arrow-key shortcuts
    pub progress_step: u32,
    /// Stop auto-progress after this many steps (0 = no limit)
    pub max_steps: u32,
    /// Steps taken since last `reset_steps()`
    pub steps_taken: u32,
}

impl Default for SceneSettings {
    fn default() -> Self {
        use crate::world::scenes::hardcoded_scenes;
        let scenes = hardcoded_scenes();
        let hardcoded_names = scenes.iter().map(|s| s.name.clone()).collect();
        Self {
            scenes,
            active_scene: 0,
            hardcoded_names,
            auto_progress: false,
            progress_rate: 1.0,
            progress_step: 1,
            max_steps: 0,
            steps_taken: 0,
        }
    }
}

impl SceneSettings {
    pub fn is_hardcoded(&self, scene: &SceneData) -> bool {
        self.hardcoded_names.contains(&scene.name)
    }

    pub fn active(&self) -> &SceneData {
        &self.scenes[self.active_scene]
    }

    pub fn active_mut(&mut self) -> &mut SceneData {
        &mut self.scenes[self.active_scene]
    }

    /// Advance the active scene's date if auto-progress is enabled.
    /// Returns the number of steps actually taken this tick (0 if none).
    /// Stops auto-progress automatically when `max_steps > 0` is reached.
    pub fn tick_progress(&mut self, dt_secs: f32) -> u32 {
        if !self.auto_progress {
            return 0;
        }
        let steps = ((dt_secs * self.progress_rate).round() as u32).max(1);

        let actual = if self.max_steps > 0 {
            steps.min(self.max_steps.saturating_sub(self.steps_taken))
        } else {
            steps
        };

        if actual > 0 {
            let advance = (actual * self.progress_step) as i32;
            let scene = self.active_mut();
            scene.date = scene.date.advance_weeks(advance);
            scene.mark_dirty();
            self.steps_taken += actual;

            if self.max_steps > 0 && self.steps_taken >= self.max_steps {
                self.auto_progress = false;
                self.steps_taken = 0;
            }
        }

        actual
    }

    pub fn reset_steps(&mut self) {
        self.steps_taken = 0;
    }
}

// ── Environment Settings ──

pub struct EnvironmentSettings {
    pub light_position: [f32; 3],
    pub ambient: f32,
    pub tropism_strength: f32,
    pub gravitropism_strength: f32,
    pub wind_azimuth: f32,
    pub wind_strength: f32,
    pub wind_turbulence: f32,
    /// When true, wind deflection is baked into geometry.
    pub wind_baked: bool,
    /// Vertex-shader sway amplitude
    pub wind_anim_strength: f32,
    pub taper: f32,
    // Proprioceptive self-correction (Bastien et al. AC model, discrete approximation).
    // Higher values produce stronger straightening toward the pre-tropism heading.
    pub proprioception_gamma: f32,
    // Ornstein–Uhlenbeck correlated heading drift (stochastic variation).
    pub variation_noise_strength: f32,
    pub variation_decay_rate: f32,
    // Per-branch colour OU drift amplitude (0 = off).
    pub colour_variation: f32,
    // Voxel-grid space pruning: branches whose tip lands in an already-occupied cell are cut.
    pub space_pruning: bool,
    pub occupancy_cell_size: f32,
    pub generation: u64,
}

impl EnvironmentSettings {
    pub fn mark_dirty(&mut self) {
        self.generation += 1;
    }
}

impl Default for EnvironmentSettings {
    fn default() -> Self {
        Self {
            light_position: [50.0, 100.0, 50.0],
            ambient: 0.3,
            // Mild phototropism and gravitropism produce naturally curved stems
            tropism_strength: 0.04,
            gravitropism_strength: 0.08,
            wind_azimuth: 45.0,
            wind_strength: 0.0,
            wind_turbulence: 0.0,
            wind_baked: true,
            wind_anim_strength: 0.0,
            taper: 0.97,
            // Gentle proprioceptive straightening toward vertical
            proprioception_gamma: 0.1,
            // Subtle correlated variation for organic branching variance
            variation_noise_strength: 0.015,
            variation_decay_rate: 0.15,
            colour_variation: 0.03,
            space_pruning: false,
            occupancy_cell_size: 0.5,
            generation: 0,
        }
    }
}

// ── Camera Settings ──

pub struct CameraSettings {
    pub speed: f32,
    pub sensitivity: f32,
    pub auto_orbit: bool,
    pub orbit_speed: f32,
    pub orbit_centre: [f32; 3],
    pub fov: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            speed: 25.0,
            sensitivity: 0.001,
            auto_orbit: false,
            orbit_speed: 0.1,
            orbit_centre: [0.0, 0.0, 0.0],
            fov: 45.0,
        }
    }
}

// ── Display Settings ──

pub struct DisplaySettings {
    pub background_colour: [f32; 3],
    pub ground_colour: [f32; 3],
    pub show_lines: bool,
    pub show_meshes: bool,
    pub debug_mode: bool,
    pub vsync: bool,
    /// Target frames per second. 0 = unlimited.
    pub frame_target: u32,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            background_colour: [0.3, 0.7, 0.9],
            ground_colour: [0.3, 0.5, 0.2],
            show_lines: false,
            show_meshes: true,
            debug_mode: false,
            vsync: true,
            frame_target: 0,
        }
    }
}

// ── LOD Settings ──

pub struct LodSettings {
    /// Distance within which full detail is rendered (8 cylinder segments).
    pub near_threshold: f32,
    /// Distance within which medium detail is rendered (5 cylinder segments).
    pub mid_threshold: f32,
    /// Distance beyond which minimum detail is rendered (3 cylinder segments).
    /// Plants further than this skip mesh generation entirely.
    pub far_threshold: f32,
    /// Fraction of leaf quads randomly skipped at near distance. 0.0 = off.
    pub leaf_skip_near: f32,
    /// Fraction of leaf quads randomly skipped at mid distance.
    pub leaf_skip_mid: f32,
    /// Fraction of leaf quads randomly skipped at far distance.
    pub leaf_skip_far: f32,
}

impl Default for LodSettings {
    fn default() -> Self {
        Self {
            near_threshold: 80.0,
            mid_threshold: 160.0,
            far_threshold: 300.0,
            leaf_skip_near: 0.0,
            leaf_skip_mid: 0.3,
            leaf_skip_far: 0.6,
        }
    }
}

// ── Cull Settings ──

pub struct CullSettings {
    /// Enable view-frustum culling. Plants outside the camera frustum are skipped entirely.
    pub frustum_culling: bool,
    /// Conservative bounding-sphere radius used for per-plant frustum culling (world units).
    /// Increase if large plants are incorrectly culled at the screen edge.
    pub cull_radius: f32,
    /// Dead-zone width (metres) around each LOD boundary to prevent oscillation.
    pub lod_hysteresis: f32,
}

impl Default for CullSettings {
    fn default() -> Self {
        Self {
            frustum_culling: true,
            cull_radius: 20.0,
            lod_hysteresis: 5.0,
        }
    }
}

impl DisplaySettings {
    pub fn clear_colour(&self) -> wgpu::Color {
        let [r, g, b] = self.background_colour;
        wgpu::Color {
            r: r as f64,
            g: g as f64,
            b: b as f64,
            a: 1.0,
        }
    }
}

// ── Ecosystem Settings ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcosystemKernel {
    Neutral,
    Inhibitory,
    Promotional,
    Mixed,
}

impl fmt::Display for EcosystemKernel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone)]
pub struct EcosystemSettings {
    pub area: f32,
    pub num_plants: u32,
    pub kernel: EcosystemKernel,
    pub kernel_radius: f32,
    pub use_self_thinning: bool,
    pub thinning_radius: f32,
    pub use_succession: bool,
    pub succession_steps: u32,
    /// Species present in this community and their relative abundance weights.
    pub species: Vec<(PlantType, f32)>,
    pub generation: u64,
}

impl EcosystemSettings {
    pub fn mark_dirty(&mut self) {
        self.generation += 1;
    }
}

impl Default for EcosystemSettings {
    fn default() -> Self {
        Self {
            area: 80.0,
            num_plants: 60,
            kernel: EcosystemKernel::Inhibitory,
            kernel_radius: 10.0,
            use_self_thinning: false,
            thinning_radius: 8.0,
            use_succession: false,
            succession_steps: 10,
            species: vec![(PlantType::Tree, 0.4), (PlantType::Fern, 0.6)],
            generation: 0,
        }
    }
}

// ── Top-level Settings ──

#[derive(Default)]
pub struct Settings {
    pub scene: SceneSettings,
    pub env: EnvironmentSettings,
    pub camera: CameraSettings,
    pub display: DisplaySettings,
    pub lod: LodSettings,
    pub cull: CullSettings,
}

// ── Season Utilities ──

/// Season name for display (season: 0=spring, 0.25=summer, 0.5=autumn, 0.75=winter).
pub fn season_name(season: f32) -> &'static str {
    let s = season.rem_euclid(1.0);
    match (s * 4.0) as u32 {
        0 => "Spring",
        1 => "Summer",
        2 => "Autumn",
        _ => "Winter",
    }
}

/// Season tint colour multiplied against plant colours in the shader.
/// Returns Vec3::ONE (no tint) in spring/summer, warm orange in autumn, cool grey in winter.
pub fn season_tint(season: f32) -> Vec3 {
    let s = season.rem_euclid(1.0);
    // Use cosine blend: summer peak at 0.25, winter trough at 0.75
    let t = 0.5 - 0.5 * (TAU * (s - 0.25)).cos(); // 0 at summer, 1 at winter
    let autumn_tint = vec3(1.0, 0.65, 0.3);
    let winter_tint = vec3(0.75, 0.75, 0.85);
    // t=0 → white (summer), t=0.5 → autumn, t=1 → winter
    if t < 0.5 {
        Vec3::ONE.lerp(autumn_tint, t * 2.0)
    } else {
        autumn_tint.lerp(winter_tint, (t - 0.5) * 2.0)
    }
}
