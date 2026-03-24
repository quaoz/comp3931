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

use crate::{
    settings::PlantType,
    util::{rng, turtle::Action},
};

/// Hermite interpolant (3t² − 2t³), clamped to `[0, 1]`.
pub fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Age-based bark colour
pub fn lerp_bark_colour(age: u32, max_age: f32, young: Vec3, old: Vec3) -> Vec3 {
    young.lerp(old, (age as f32 / max_age).min(1.0))
}

/// Seasonal dormancy factor
pub fn dormancy_factor(season: f32, offset: f32, max_dormancy: f32) -> f32 {
    const ONSET: f32 = 0.50; // mid-autumn
    const FULL: f32 = 0.82; // mid-winter
    const RECOVER: f32 = 0.90; // late-winter

    let s = (season + offset).rem_euclid(1.0);
    if s < ONSET {
        0.0
    } else if s < FULL {
        max_dormancy * smoothstep((s - ONSET) / (FULL - ONSET))
    } else if s < RECOVER {
        max_dormancy
    } else {
        max_dormancy * (1.0 - smoothstep((s - RECOVER) / (1.0 - RECOVER)))
    }
}

/// Foliage colour shifting
pub fn autumn_colour(season: f32, summer: Vec3, autumn: Vec3, start: f32, end: f32) -> Vec3 {
    summer.lerp(autumn, smoothstep((season - start) / (end - start)))
}

/// Early-spring growth surge
pub fn spring_surge(season: f32) -> f32 {
    const PEAK: f32 = 0.08;
    const END: f32 = 0.18;

    if season < PEAK {
        smoothstep(season / PEAK)
    } else if season < END {
        1.0 - smoothstep((season - PEAK) / (END - PEAK))
    } else {
        0.0
    }
}

// ── Environment ──

#[derive(Debug, Clone, Copy)]
pub struct PlantEnvironment {
    pub season: f32,
}

// ── Plant trait ──

pub trait Plant: Send {
    fn plant_type(&self) -> PlantType;
    fn iteration(&self) -> u32;
    fn max_iterations(&self) -> u32;
    fn set_iteration(&mut self, iteration: u32);
    fn actions(&mut self, env: &PlantEnvironment) -> &[Action];

    /// Draw plant-specific UI controls, returns true if the scene needs to be rebuilt
    fn ui(&mut self, ui: &mut egui::Ui) -> bool;
    fn colour(&self) -> Vec3;
    fn clone_boxed(&self) -> Box<dyn Plant>;
}

// ── Species ──

pub trait Species: Send + 'static {
    const TYPE: PlantType;
    type Params: Clone + Default + Send;

    fn generate(age: u32, params: &Self::Params, season: f32, dormancy_offset: f32) -> Vec<Action>;
    fn max_iterations(params: &Self::Params) -> u32;
    fn colour(params: &Self::Params, iteration: u32) -> Vec3;
    fn ui(params: &mut Self::Params, ui: &mut egui::Ui) -> bool;
}

const INITIAL_SEASON: f32 = 0.25;
const SEASON_EPSILON: f32 = 0.02;

/// An instance of some [`Species`]
pub struct PlantBase<S: Species> {
    iteration: u32,
    dirty: bool,
    cached_actions: Vec<Action>,
    params: S::Params,
    last_season: f32,
    dormancy_offset: f32,
}

impl<S: Species> Default for PlantBase<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Species> PlantBase<S> {
    pub fn new() -> Self {
        let params = S::Params::default();
        let dormancy_offset = rng::random_range(-0.05, 0.05);
        Self {
            iteration: 0,
            dirty: false,
            cached_actions: S::generate(0, &params, INITIAL_SEASON, dormancy_offset),
            params,
            last_season: INITIAL_SEASON,
            dormancy_offset,
        }
    }
}

impl<S: Species> Plant for PlantBase<S> {
    fn plant_type(&self) -> PlantType {
        S::TYPE
    }

    fn iteration(&self) -> u32 {
        self.iteration
    }

    fn max_iterations(&self) -> u32 {
        S::max_iterations(&self.params)
    }

    fn set_iteration(&mut self, iteration: u32) {
        if self.iteration != iteration {
            self.iteration = iteration;
            self.dirty = true;
        }
    }

    fn colour(&self) -> Vec3 {
        S::colour(&self.params, self.iteration)
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        let changed = S::ui(&mut self.params, ui);
        self.dirty |= changed;
        changed
    }

    fn actions(&mut self, env: &PlantEnvironment) -> &[Action] {
        if (env.season - self.last_season).abs() > SEASON_EPSILON {
            self.dirty = true;
        }
        if self.dirty {
            self.cached_actions = S::generate(
                self.iteration,
                &self.params,
                env.season,
                self.dormancy_offset,
            );
            self.last_season = env.season;
            self.dirty = false;
        }
        &self.cached_actions
    }

    fn clone_boxed(&self) -> Box<dyn Plant> {
        // Actions are left empty and rebuilt on next access rather than cloned
        Box::new(Self {
            iteration: self.iteration,
            dirty: true,
            cached_actions: Vec::new(),
            params: self.params.clone(),
            last_season: self.last_season,
            dormancy_offset: self.dormancy_offset,
        })
    }
}

// ── PlantInstance wrapper ──

pub struct PlantInstance {
    pub position: Vec3,
    pub scale: f32,
    pub rotation: f32,
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
    fn boxed<S: Species>() -> Box<dyn Plant> {
        Box::new(PlantBase::<S>::new())
    }

    match plant_type {
        PlantType::Tree => boxed::<tree::Tree>(),
        PlantType::Bush => boxed::<bush::Bush>(),
        PlantType::Fern => boxed::<fern::Fern>(),
        PlantType::Wildflower => boxed::<wildflower::Wildflower>(),
        PlantType::Capsella => boxed::<capsella::Capsella>(),
        PlantType::Mint => boxed::<mint::Mint>(),
        PlantType::Lychnis => boxed::<lychnis::Lychnis>(),
        PlantType::Mycelis => boxed::<mycelis::Mycelis>(),
        PlantType::Carrot => boxed::<carrot::Carrot>(),
    }
}

#[cfg(test)]
mod tests {
    use glam::{Vec3, vec3};

    use super::*;
    use crate::util::rng;

    fn summer() -> PlantEnvironment {
        PlantEnvironment { season: 0.25 }
    }

    // ── dormancy_factor ──

    #[test]
    fn dormancy_zero_before_onset() {
        // Season in [0, 0.50) → no dormancy at all.
        assert_eq!(dormancy_factor(0.0, 0.0, 0.65), 0.0);
        assert_eq!(dormancy_factor(0.25, 0.0, 0.65), 0.0); // midsummer
        assert_eq!(dormancy_factor(0.49, 0.0, 0.65), 0.0); // just before onset
    }

    #[test]
    fn dormancy_max_in_full_winter() {
        // Season in [0.82, 0.90) → at peak dormancy for the whole plateau.
        let max = 0.65_f32;
        assert!((dormancy_factor(0.82, 0.0, max) - max).abs() < 1e-5);
        assert!((dormancy_factor(0.86, 0.0, max) - max).abs() < 1e-5);
        assert!((dormancy_factor(0.89, 0.0, max) - max).abs() < 1e-5);
    }

    #[test]
    fn dormancy_smoothstep_at_onset_midpoint() {
        // Midpoint of onset ramp: s = (0.50 + 0.82) / 2 = 0.66
        // t = 0.5, smoothstep(0.5) = 0.5 → result = max * 0.5
        let v = dormancy_factor(0.66, 0.0, 0.65);
        assert!(
            (v - 0.65 * 0.5).abs() < 1e-5,
            "expected {}, got {v}",
            0.65 * 0.5
        );
    }

    #[test]
    fn dormancy_smoothstep_at_recovery_midpoint() {
        // Midpoint of recovery ramp: s = (0.90 + 1.0) / 2 = 0.95
        // t = 0.5, smoothstep(0.5) = 0.5 → result = max * (1 - 0.5)
        let v = dormancy_factor(0.95, 0.0, 0.65);
        assert!(
            (v - 0.65 * 0.5).abs() < 1e-5,
            "expected {}, got {v}",
            0.65 * 0.5
        );
    }

    #[test]
    fn dormancy_wraps_at_season_one() {
        // season=1.0 → rem_euclid(1.0) = 0.0 → same as season=0.0 (spring, no dormancy).
        assert_eq!(
            dormancy_factor(1.0, 0.0, 0.65),
            dormancy_factor(0.0, 0.0, 0.65)
        );
    }

    #[test]
    fn dormancy_positive_offset_shifts_onset_earlier() {
        // At season=0.48 with no offset the effective season is below ONSET (0.50) → 0.
        assert_eq!(dormancy_factor(0.48, 0.0, 0.65), 0.0);
        // With +0.05 offset the effective season is 0.53 > ONSET → non-zero.
        assert!(dormancy_factor(0.48, 0.05, 0.65) > 0.0);
    }

    #[test]
    fn dormancy_offset_wraps_across_year_boundary() {
        // season=0.98 + offset=0.08 → 1.06 rem_euclid 1.0 = 0.06 → spring → 0.
        assert_eq!(dormancy_factor(0.98, 0.08, 0.65), 0.0);
    }

    // ── lerp_bark_colour ──

    #[test]
    fn lerp_at_age_zero_returns_young() {
        let young = vec3(1.0, 0.0, 0.0);
        let old = vec3(0.0, 0.0, 1.0);
        assert_eq!(lerp_bark_colour(0, 10.0, young, old), young);
    }

    #[test]
    fn lerp_at_max_age_returns_old() {
        let young = vec3(1.0, 0.0, 0.0);
        let old = vec3(0.0, 0.0, 1.0);
        assert_eq!(lerp_bark_colour(10, 10.0, young, old), old);
    }

    #[test]
    fn lerp_at_half_age_is_midpoint() {
        let young = vec3(0.0, 0.0, 0.0);
        let old = vec3(1.0, 1.0, 1.0);
        let mid = lerp_bark_colour(5, 10.0, young, old);
        assert!((mid - vec3(0.5, 0.5, 0.5)).length() < 1e-5);
    }

    #[test]
    fn lerp_clamps_past_max_age() {
        let young = vec3(1.0, 0.0, 0.0);
        let old = vec3(0.0, 0.0, 1.0);
        // age > max_age should clamp to old colour
        assert_eq!(lerp_bark_colour(20, 10.0, young, old), old);
    }

    // ── Smoke tests: all plants produce actions ──

    #[test]
    fn all_plants_produce_actions_at_iteration_1() {
        rng::seed(42);
        let env = summer();
        for &pt in &PlantType::ALL {
            let mut inst = PlantInstance::new(pt, 0.0);
            inst.plant.set_iteration(1);
            let actions = inst.plant.actions(&env);
            assert!(
                !actions.is_empty(),
                "{pt:?} produced no actions at iteration 1"
            );
        }
    }

    #[test]
    fn all_plants_produce_actions_at_iteration_0() {
        rng::seed(42);
        let env = summer();
        for &pt in &PlantType::ALL {
            let mut inst = PlantInstance::new(pt, 0.0);
            let actions = inst.plant.actions(&env);
            assert!(
                !actions.is_empty(),
                "{pt:?} produced no actions at iteration 0"
            );
        }
    }

    // ── set_iteration triggers cache refresh ──

    #[test]
    fn set_iteration_to_same_value_does_not_change_actions() {
        rng::seed(42);
        let env = summer();
        let mut inst = PlantInstance::new(PlantType::Lychnis, 0.0);
        inst.plant.set_iteration(3);
        let count_before = inst.plant.actions(&env).len();
        inst.plant.set_iteration(3); // same iteration again — should not invalidate
        let count_after = inst.plant.actions(&env).len();
        assert_eq!(count_before, count_after);
    }

    #[test]
    fn plant_type_reported_correctly() {
        for &pt in &PlantType::ALL {
            let inst = PlantInstance::new(pt, 0.0);
            assert_eq!(inst.plant.plant_type(), pt);
        }
    }

    #[test]
    fn plant_iteration_reported_correctly() {
        rng::seed(0);
        for iter in [0u32, 1, 3, 5] {
            let mut inst = PlantInstance::new(PlantType::Fern, 0.0);
            inst.plant.set_iteration(iter);
            assert_eq!(inst.plant.iteration(), iter);
        }
    }

    #[test]
    fn plant_max_iterations_positive() {
        for &pt in &PlantType::ALL {
            assert!(PlantInstance::new(pt, 0.0).plant.max_iterations() > 0);
        }
    }

    // ── PlantInstance construction & transform ──

    #[test]
    fn plant_instance_default_values() {
        let inst = PlantInstance::new(PlantType::Tree, 5.0);
        assert_eq!(inst.position, Vec3::ZERO);
        assert_eq!(inst.scale, 1.0);
        assert_eq!(inst.rotation, 0.0);
        assert_eq!(inst.delay_years, 5.0);
    }

    #[test]
    fn plant_instance_with_transform_sets_fields() {
        let inst =
            PlantInstance::new(PlantType::Fern, 1.0).with_transform(vec3(1.0, 0.0, 2.0), 3.0, 45.0);
        assert_eq!(inst.position, vec3(1.0, 0.0, 2.0));
        assert_eq!(inst.scale, 3.0);
        assert_eq!(inst.rotation, 45.0);
        assert_eq!(inst.delay_years, 1.0);
    }

    #[test]
    fn plant_instance_clone_preserves_fields() {
        let inst =
            PlantInstance::new(PlantType::Mint, 2.5).with_transform(vec3(3.0, 0.0, 4.0), 2.0, 90.0);
        let cloned = inst.clone();
        assert_eq!(cloned.position, inst.position);
        assert_eq!(cloned.scale, inst.scale);
        assert_eq!(cloned.rotation, inst.rotation);
        assert_eq!(cloned.delay_years, inst.delay_years);
        assert_eq!(cloned.plant.plant_type(), inst.plant.plant_type());
    }

    #[test]
    fn plant_instance_with_transform_preserves_delay() {
        let inst = PlantInstance::new(PlantType::Bush, 7.3).with_transform(Vec3::ZERO, 1.0, 0.0);
        assert_eq!(inst.delay_years, 7.3);
    }

    #[test]
    fn plant_properties_consistent() {
        rng::seed(0);
        for &pt in &PlantType::ALL {
            let inst = PlantInstance::new(pt, 0.0);
            assert_eq!(inst.plant.plant_type(), pt);
            assert!(
                inst.plant.max_iterations() > 0,
                "{pt:?} max_iterations must be positive"
            );
            assert!(
                pt.years_per_iteration() > 0.0,
                "{pt:?} years_per_iteration must be positive"
            );
            assert!(
                (0.0..=1.0).contains(&pt.shade_tolerance()),
                "{pt:?} shade_tolerance out of range"
            );
        }
    }
}
