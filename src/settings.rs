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
}

impl fmt::Display for PlantType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlantType::Tree => write!(f, "Tree"),
            PlantType::Bush => write!(f, "Bush"),
            PlantType::Fern => write!(f, "Fern"),
            PlantType::Wildflower => write!(f, "Wildflower"),
            PlantType::Capsella => write!(f, "Capsella"),
            PlantType::Mint => write!(f, "Mint"),
            PlantType::Lychnis => write!(f, "Lychnis"),
            PlantType::Mycelis => write!(f, "Mycelis"),
            PlantType::Carrot => write!(f, "Carrot"),
        }
    }
}

// ── Scene Data ──

#[derive(Debug, Clone)]
pub struct SceneData {
    pub name: String,
    pub plants: Vec<PlantInstance>,
    pub global_scale: f32,
    pub global_age: u32,
    pub dirty: bool,
}

// ── Scene Settings ──

#[derive(Debug, Clone)]
pub struct SceneSettings {
    pub scenes: Vec<SceneData>,
    pub active_scene: usize,
    pub hardcoded_names: HashSet<String>,
}

impl Default for SceneSettings {
    fn default() -> Self {
        use crate::world::scenes::hardcoded_scenes;
        let scenes = hardcoded_scenes();
        let hardcoded_names = scenes.iter().map(|s| s.name.clone()).collect();
        Self {
            scenes,
            active_scene: 1,
            hardcoded_names,
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
}

// ── Camera Settings ──

pub struct CameraSettings {
    pub speed: f32,
    pub sensitivity: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            speed: 25.0,
            sensitivity: 0.001,
        }
    }
}

// ── Display Settings ──

pub struct DisplaySettings {
    pub background_color: [f32; 3],
    pub ground_color: [f32; 3],
    pub fov: f32,
    pub show_lines: bool,
    pub show_meshes: bool,
    pub debug_mode: bool,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            background_color: [0.3, 0.7, 0.9],
            ground_color: [0.3, 0.5, 0.2],
            fov: 45.0,
            show_lines: false,
            show_meshes: true,
            debug_mode: false,
        }
    }
}

impl DisplaySettings {
    pub fn clear_color(&self) -> wgpu::Color {
        let [r, g, b] = self.background_color;
        wgpu::Color {
            r: r as f64,
            g: g as f64,
            b: b as f64,
            a: 1.0,
        }
    }
}

// ── Top-level Settings ──

#[derive(Default)]
pub struct Settings {
    pub scene: SceneSettings,
    pub camera: CameraSettings,
    pub display: DisplaySettings,
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

/// Growth factor [0.3, 1.0] based on season. Summer=1.0, Winter=0.3.
pub fn season_growth_factor(season: f32) -> f32 {
    let s = season.rem_euclid(1.0);
    // Cosine: peak at summer (0.25), trough at winter (0.75)
    0.3 + 0.7 * (0.5 + 0.5 * (TAU * (s - 0.25)).cos())
}

/// Returns true if the growth factor would round to a different effective age,
/// used to avoid full rebuilds every frame during auto-advance.
pub fn season_needs_rebuild(old: f32, new: f32) -> bool {
    (old * 20.0) as u32 != (new * 20.0) as u32
}
