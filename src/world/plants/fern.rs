//! Fern plant inspired by Campbell (1996) "Describing shapes of ferns using L-System models".

use std::{
    f32::consts::{FRAC_PI_2, FRAC_PI_3, FRAC_PI_6},
    fmt::Display,
};

use glam::{Vec3, vec3};

use crate::{
    settings::PlantType,
    util::{
        lsystem::{LSystem, Rule, Symbol, SymbolType},
        rng,
        turtle::Action,
    },
    world::plants::{Plant, PlantEnvironment},
};

#[derive(Clone)]
pub struct FernParams {
    pub colour: Vec3,
    pub leaf_width: f32,
    pub leaf_height: f32,
    pub branch_angle_deg: f32,
    pub branch_droop_deg: f32,
    pub branch_base_len: f32,
    pub max_iterations: u32,
}

impl Default for FernParams {
    fn default() -> Self {
        Self {
            colour: vec3(0.47, 0.78, 0.2),
            leaf_width: 0.08,
            leaf_height: 0.09,
            branch_angle_deg: 60.0,
            branch_droop_deg: -10.0,
            branch_base_len: 0.02,
            max_iterations: 25,
        }
    }
}

pub struct FernPlant {
    iteration: u32,
    dirty: bool,
    cached_actions: Vec<Action>,
    pub params: FernParams,
    last_season: f32,
    dormancy_offset: f32,
}

impl Default for FernPlant {
    fn default() -> Self {
        Self::new()
    }
}

impl FernPlant {
    pub fn new() -> Self {
        let params = FernParams::default();
        let dormancy_offset = rng::random_range(-0.05, 0.05);
        let actions = generate(0, &params, 0.25, dormancy_offset);
        Self {
            iteration: 0,
            dirty: false,
            cached_actions: actions,
            params,
            last_season: 0.25,
            dormancy_offset,
        }
    }
}

impl Plant for FernPlant {
    fn plant_type(&self) -> PlantType {
        PlantType::Fern
    }

    fn iteration(&self) -> u32 {
        self.iteration
    }

    fn max_iterations(&self) -> u32 {
        self.params.max_iterations
    }

    fn set_iteration(&mut self, iteration: u32) {
        if self.iteration != iteration {
            self.iteration = iteration;
            self.dirty = true;
        }
    }

    fn colour(&self) -> Vec3 {
        self.params.colour
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        let p = &mut self.params;

        egui::Grid::new("fern_params")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("Max iterations");
                if ui
                    .add(egui::DragValue::new(&mut p.max_iterations).range(1..=40))
                    .changed()
                {
                    self.dirty = true;
                }
                ui.end_row();

                ui.label("Colour");
                let mut rgb = p.colour.to_array();
                if ui.color_edit_button_rgb(&mut rgb).changed() {
                    p.colour = Vec3::from(rgb);
                    self.dirty = true;
                }
                ui.end_row();

                ui.label("Branch angle");
                if ui
                    .add(egui::Slider::new(&mut p.branch_angle_deg, 10.0..=90.0).suffix("°"))
                    .changed()
                {
                    self.dirty = true;
                }
                ui.end_row();

                ui.label("Frond droop");
                if ui
                    .add(egui::Slider::new(&mut p.branch_droop_deg, -30.0..=0.0).suffix("°"))
                    .changed()
                {
                    self.dirty = true;
                }
                ui.end_row();

                ui.label("Branch length");
                self.dirty |= ui
                    .add(egui::Slider::new(&mut p.branch_base_len, 0.005..=0.1).max_decimals(3))
                    .changed();
                ui.end_row();

                ui.label("Leaf width");
                self.dirty |= ui
                    .add(egui::Slider::new(&mut p.leaf_width, 0.01..=0.3).max_decimals(3))
                    .changed();
                ui.end_row();

                ui.label("Leaf height");
                self.dirty |= ui
                    .add(egui::Slider::new(&mut p.leaf_height, 0.01..=0.3).max_decimals(3))
                    .changed();
                ui.end_row();
            });

        if ui.button("Reset").clicked() {
            self.params = FernParams::default();
            self.dirty = true;
        }

        self.dirty
    }

    fn clone_boxed(&self) -> Box<dyn Plant> {
        let mut p = Self::new();
        p.params = self.params.clone();
        p.iteration = self.iteration;
        p.last_season = self.last_season;
        p.dormancy_offset = self.dormancy_offset;
        p.dirty = true;
        Box::new(p)
    }

    fn actions(&mut self, env: &PlantEnvironment) -> &[Action] {
        if (env.season - self.last_season).abs() > 0.02 {
            self.dirty = true;
        }
        if self.dirty {
            self.cached_actions = generate(
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
}

#[derive(Debug, Copy, Clone)]
enum Fs {
    Leaf(u8, u8, u8), // age, dist, depth
    Branch(u8, u8),   // age, depth
    Push,
    Pop,
    Roll(f32),
    Turn(f32),
    Pitch(f32),
    Layer(u8, u8), // delay, depth
}

impl PartialEq for Fs {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}

impl Symbol for Fs {
    fn symbol_type(&self) -> SymbolType {
        match self {
            Fs::Layer(..) | Fs::Leaf(..) | Fs::Branch(..) => SymbolType::NonTerminal,
            _ => SymbolType::Terminal,
        }
    }
}

impl Display for Fs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Leaf(age, dist, depth) => write!(f, "L({age}, {dist}, {depth})"),
            Self::Branch(age, depth) => write!(f, "B({age}, {depth})"),
            Self::Push => write!(f, "["),
            Self::Pop => write!(f, "]"),
            Self::Roll(angle) => {
                if *angle >= 0.0 {
                    write!(f, "/({angle})")
                } else {
                    write!(f, "\\textbackslash({})", angle.abs())
                }
            }
            Self::Turn(angle) => {
                if *angle >= 0.0 {
                    write!(f, "+({angle})")
                } else {
                    write!(f, "-({})", angle.abs())
                }
            }
            Self::Pitch(angle) => {
                if *angle >= 0.0 {
                    write!(f, "^({angle})")
                } else {
                    write!(f, "&({})", angle.abs())
                }
            }
            Self::Layer(delay, depth) => write!(f, "C({delay}, {depth})"),
        }
    }
}

fn generate(age: u32, p: &FernParams, season: f32, dormancy_offset: f32) -> Vec<Action> {
    // Ferns die back in autumn/winter; recover in early spring.
    // Max dormancy 0.65: some fronds persist through winter.
    let age =
        (age as f32 * (1.0 - super::dormancy_factor(season, dormancy_offset, 0.65))).round() as u32;
    use Fs::*;

    // Autumn frond browning: fronds turn from green to brown as they die back.
    let autumn_frond = glam::vec3(0.55, 0.38, 0.10);
    let frond_colour = if season < 0.45 {
        p.colour
    } else if season < 0.82 {
        let t = (season - 0.45) / 0.37;
        let t = t * t * (3.0 - 2.0 * t);
        p.colour.lerp(autumn_frond, t)
    } else {
        autumn_frond
    };

    let leaf_width = p.leaf_width;
    let leaf_height = p.leaf_height;
    let leaf_max_age: u8 = 7;
    let leaf_max_dist: u8 = 9;

    let branch_max_depth: u8 = 3;
    let branch_max_age: u8 = 7;
    let branch_angle = p.branch_angle_deg.to_radians();
    let branch_droop = p.branch_droop_deg.to_radians();
    let branch_base_len = p.branch_base_len;

    // Leaf tick: non-branching leaves increment age (or stay fixed at max).
    let rule_a_leaf_grow = Rule::Parametric(Leaf(0, 0, 0), &move |s: &Fs, out: &mut Vec<Fs>| {
        if let &Fs::Leaf(age, dist, depth) = s
            && age < leaf_max_age
            && (age < 3 || dist > leaf_max_dist || depth > 2)
        {
            out.push(Leaf(age + 1, dist, depth));
            true
        } else {
            false
        }
    });

    // Leaf branch: branching leaves (age in [3,7], dist <= 9, depth <= 2).
    let rule_a_leaf_branch = Rule::Parametric(Leaf(0, 0, 0), &move |s: &Fs, out: &mut Vec<Fs>| {
        if let &Fs::Leaf(age, dist, depth) = s
            && age >= 3
            && age <= leaf_max_age
            && dist <= leaf_max_dist
            && depth <= 2
        {
            let depth_scale = (branch_max_depth - depth) as f32 / branch_max_depth as f32;
            let dist_scale = (2 * dist) as f32 / leaf_max_dist as f32;
            out.extend([
                Pitch(branch_droop * (depth_scale + dist_scale)),
                Branch(1, depth),
                Push,
                Turn(-branch_angle),
                Leaf(2, dist, depth + 1),
                Pop,
                Push,
                Branch(1, depth),
                Leaf(age, dist + 1, depth),
                Pop,
                Push,
                Turn(branch_angle),
                Leaf(2, dist, depth + 1),
                Pop,
            ]);
            true
        } else {
            false
        }
    });

    // Branch: age increments up to branch_max_age then stays fixed.
    let rule_b = Rule::Parametric(Branch(0, 0), &move |s: &Fs, out: &mut Vec<Fs>| {
        if let &Fs::Branch(age, depth) = s
            && age <= branch_max_age
        {
            out.push(Branch(age + 1, depth));
            true
        } else {
            false
        }
    });

    // Layer tick: delay > 0 — countdown to expansion.
    let rule_c_tick = Rule::Parametric(Layer(0, 0), &|s: &Fs, out: &mut Vec<Fs>| {
        if let &Fs::Layer(delay, depth) = s
            && delay > 0
        {
            out.push(Layer(delay - 1, depth));
            true
        } else {
            false
        }
    });

    // Layer expand: delay == 0 — emit three fronds then recurse deeper.
    let rule_c_expand = Rule::Parametric(Layer(0, 0), &move |s: &Fs, out: &mut Vec<Fs>| {
        if let &Fs::Layer(delay, depth) = s
            && delay == 0
        {
            let pitch = (depth as f32).sqrt() * FRAC_PI_6;
            out.extend([
                Push,
                Pitch(pitch),
                Leaf(3, depth, 1),
                Pop,
                Turn(2.0 * FRAC_PI_3),
                Push,
                Pitch(pitch),
                Leaf(3, depth, 1),
                Pop,
                Turn(2.0 * FRAC_PI_3),
                Push,
                Pitch(pitch),
                Leaf(3, depth, 1),
                Pop,
                Turn(2.0 * FRAC_PI_3 / (depth + 1) as f32),
                Layer(3, depth + 2),
            ]);
            true
        } else {
            false
        }
    });

    let mut lsystem: LSystem<Fs> = LSystem::new(
        &[
            Roll(FRAC_PI_2),
            Turn(-FRAC_PI_2),
            Roll(-FRAC_PI_2),
            Layer(0, 1),
        ],
        vec![
            rule_a_leaf_grow,
            rule_a_leaf_branch,
            rule_b,
            rule_c_tick,
            rule_c_expand,
        ],
    );
    lsystem.evolve(age as usize);

    let mut actions = vec![Action::Colour(frond_colour)];
    actions.extend(lsystem.current().iter().map(|&s| match s {
        Leaf(age, ..) => {
            let scale = age as f32 / leaf_max_age as f32;
            Action::Leaf(leaf_width * scale, leaf_height * scale)
        }
        Branch(age, depth) => {
            let depth_scale = (branch_max_depth - depth) as f32 / branch_max_depth as f32;
            let age_scale = age as f32 / branch_max_age as f32;
            Action::Branch(
                branch_base_len * age_scale * (branch_max_depth as f32 - depth as f32).powi(2),
                0.01 * (depth_scale + age_scale),
            )
        }
        Roll(angle) => Action::Roll(angle),
        Turn(angle) => Action::Turn(angle),
        Pitch(angle) => Action::Pitch(angle),
        Push => Action::Push,
        Pop => Action::Pop,
        Layer(..) => Action::Nop,
    }));
    actions
}
