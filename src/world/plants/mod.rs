pub mod bush;
pub mod capsella;
pub mod carrot;
pub mod fern;
pub mod lychnis;
pub mod mint;
pub mod mycelis;
pub mod tree;
pub mod wildflower;

use glam::Vec3;

use crate::{settings::PlantType, util::turtle::Action};

/// Age-based bark colour, interpolating from `young` to `old` over `max_age` years.
pub fn lerp_bark_colour(age: u32, max_age: f32, young: Vec3, old: Vec3) -> Vec3 {
    young.lerp(old, (age as f32 / max_age).min(1.0))
}

/// Seasonal dormancy factor in `[0, max_dormancy]`.
///
/// Onset mid-autumn (season ≈ 0.50), full dormancy by mid-winter (0.82),
/// partial recovery late-winter (0.90), fully recovered at spring (1.0 ≡ 0.0).
/// `offset` is a per-plant phase shift in (−0.05, +0.05) that staggers onset
/// and recovery across individuals of the same species.
pub fn dormancy_factor(season: f32, offset: f32, max_dormancy: f32) -> f32 {
    const ONSET: f32 = 0.50;
    const FULL: f32 = 0.82;
    const RECOVER: f32 = 0.90;
    let s = (season + offset).rem_euclid(1.0);
    if s < ONSET {
        0.0
    } else if s < FULL {
        let t = (s - ONSET) / (FULL - ONSET);
        max_dormancy * t * t * (3.0 - 2.0 * t)
    } else if s < RECOVER {
        max_dormancy
    } else {
        let t = (s - RECOVER) / (1.0 - RECOVER);
        max_dormancy * (1.0 - t * t * (3.0 - 2.0 * t))
    }
}

// ── Environment ──

/// Environmental context passed to each plant during geometry generation.
/// Seasonal and other ambient properties are read directly from here so
/// plants can express them as L-system rules rather than cached scalar fields.
#[derive(Debug, Clone, Copy)]
pub struct PlantEnvironment {
    /// Fractional season in [0, 1): 0 = spring, 0.25 = summer, 0.5 = autumn, 0.75 = winter.
    pub season: f32,
}

// ── Plant trait ──

pub trait Plant: Send {
    fn plant_type(&self) -> PlantType;
    fn iteration(&self) -> u32;
    fn max_iterations(&self) -> u32;
    fn set_iteration(&mut self, iteration: u32);
    fn actions(&mut self, env: &PlantEnvironment) -> &[Action];

    /// Draw plant-specific UI controls. Returns true if the scene needs to be rebuilt.
    fn ui(&mut self, ui: &mut egui::Ui) -> bool;
    fn colour(&self) -> Vec3;
    fn clone_boxed(&self) -> Box<dyn Plant>;
}

// ── PlantInstance wrapper ──

pub struct PlantInstance {
    pub position: Vec3,
    pub scale: f32,
    pub rotation: f32,
    /// Random delay in years before this plant starts growing. The plant's
    /// effective iteration is computed from `(scene_total_years - delay_years).max(0)`.
    pub delay_years: f32,
    pub plant: Box<dyn Plant>,
}

impl PlantInstance {
    pub fn new(plant_type: PlantType, delay_years: f32) -> Self {
        Self {
            position: Vec3::ZERO,
            scale: 1.0,
            rotation: 0.0,
            delay_years,
            plant: create_plant(plant_type),
        }
    }

    pub fn with_transform(mut self, position: impl Into<Vec3>, scale: f32, rotation: f32) -> Self {
        self.position = position.into();
        self.scale = scale;
        self.rotation = rotation;
        self
    }
}

impl Clone for PlantInstance {
    fn clone(&self) -> Self {
        Self {
            position: self.position,
            scale: self.scale,
            rotation: self.rotation,
            delay_years: self.delay_years,
            plant: self.plant.clone_boxed(),
        }
    }
}

impl std::fmt::Debug for PlantInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlantInstance")
            .field("plant_type", &self.plant.plant_type())
            .field("delay_years", &self.delay_years)
            .field("position", &self.position)
            .field("scale", &self.scale)
            .field("rotation", &self.rotation)
            .finish()
    }
}

fn create_plant(plant_type: PlantType) -> Box<dyn Plant> {
    match plant_type {
        PlantType::Tree => Box::new(tree::TreePlant::new()),
        PlantType::Bush => Box::new(bush::BushPlant::new()),
        PlantType::Fern => Box::new(fern::FernPlant::new()),
        PlantType::Wildflower => Box::new(wildflower::WildflowerPlant::new()),
        PlantType::Capsella => Box::new(capsella::CapsellaPlant::new()),
        PlantType::Mint => Box::new(mint::MintPlant::new()),
        PlantType::Lychnis => Box::new(lychnis::LychnisPlant::new()),
        PlantType::Mycelis => Box::new(mycelis::MycelisPlant::new()),
        PlantType::Carrot => Box::new(carrot::CarrotPlant::new()),
    }
}
