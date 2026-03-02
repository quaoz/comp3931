//! Wildflower — stochastic flowering plant with golden-angle phyllotaxis.

use std::fmt::Display;

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
pub struct WildflowerParams {
    pub stem_colour: Vec3,
    pub flower_colour: Vec3,
    pub leaf_colour: Vec3,
    pub branch_radius: f32,
    pub branch_len: f32,
    pub flower_size: f32,
    pub leaf_size: f32,
    pub branch_angle_deg: f32,
    pub max_iterations: u32,
}

impl Default for WildflowerParams {
    fn default() -> Self {
        Self {
            stem_colour: vec3(0.2, 0.71, 0.2),
            flower_colour: vec3(1.0, 0.0, 1.0),
            leaf_colour: vec3(0.39, 0.98, 0.0),
            branch_radius: 0.02,
            branch_len: 0.1,
            flower_size: 0.04,
            leaf_size: 0.02,
            branch_angle_deg: 40.0,
            max_iterations: 20,
        }
    }
}

pub struct WildflowerPlant {
    iteration: u32,
    dirty: bool,
    cached_actions: Vec<Action>,
    pub params: WildflowerParams,
    last_season: f32,
    dormancy_offset: f32,
}

impl Default for WildflowerPlant {
    fn default() -> Self {
        Self::new()
    }
}

impl WildflowerPlant {
    pub fn new() -> Self {
        let params = WildflowerParams::default();
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

impl Plant for WildflowerPlant {
    fn plant_type(&self) -> PlantType {
        PlantType::Wildflower
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
        self.params.stem_colour
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        let p = &mut self.params;

        egui::Grid::new("wildflower_params")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("Max iterations");
                let mut a = p.max_iterations as i32;
                if ui.add(egui::DragValue::new(&mut a).range(1..=30)).changed() {
                    p.max_iterations = a as u32;
                    changed = true;
                }
                ui.end_row();

                ui.label("Branch angle");
                if ui
                    .add(egui::Slider::new(&mut p.branch_angle_deg, 5.0..=80.0).suffix("°"))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Branch length");
                if ui
                    .add(egui::Slider::new(&mut p.branch_len, 0.02..=0.3).max_decimals(3))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Branch radius");
                if ui
                    .add(egui::Slider::new(&mut p.branch_radius, 0.002..=0.06).max_decimals(4))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Flower size");
                if ui
                    .add(egui::Slider::new(&mut p.flower_size, 0.005..=0.12).max_decimals(3))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Leaf size");
                if ui
                    .add(egui::Slider::new(&mut p.leaf_size, 0.005..=0.08).max_decimals(3))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Stem colour");
                let mut rgb = p.stem_colour.to_array();
                if ui.color_edit_button_rgb(&mut rgb).changed() {
                    p.stem_colour = Vec3::from(rgb);
                    changed = true;
                }
                ui.end_row();

                ui.label("Flower colour");
                let mut rgb = p.flower_colour.to_array();
                if ui.color_edit_button_rgb(&mut rgb).changed() {
                    p.flower_colour = Vec3::from(rgb);
                    changed = true;
                }
                ui.end_row();

                ui.label("Leaf colour");
                let mut rgb = p.leaf_colour.to_array();
                if ui.color_edit_button_rgb(&mut rgb).changed() {
                    p.leaf_colour = Vec3::from(rgb);
                    changed = true;
                }
                ui.end_row();
            });

        if ui.button("Reset").clicked() {
            self.params = WildflowerParams::default();
            changed = true;
        }
        if changed {
            self.dirty = true;
        }
        changed
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
enum Wf {
    P,
    A(f32),
    I(f32),
    B,
    F,
    Turn(f32),
    Roll(f32),
    Colour(Vec3),
    Leaf(f32, f32),
    Push,
    Pop,
}

impl Display for Wf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::P => write!(f, "P"),
            Self::A(a) => write!(f, "A({})", a),
            Self::I(i) => write!(f, "I({})", i),
            Self::B => write!(f, "B",),
            Self::F => write!(f, "F"),
            Self::Turn(angle) => {
                if *angle >= 0f32 {
                    write!(f, "+({angle})")
                } else {
                    write!(f, "-({angle})")
                }
            }
            Self::Roll(angle) => {
                if *angle >= 0f32 {
                    write!(f, "/({angle})")
                } else {
                    write!(f, "\\({angle})")
                }
            }
            Self::Colour(vec3) => write!(f, "C({})", vec3),
            Self::Leaf(w, h) => write!(f, "L({w}, {h})"),
            Self::Push => write!(f, "["),
            Self::Pop => write!(f, "]"),
        }
    }
}

impl PartialEq for Wf {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Symbol for Wf {
    fn symbol_type(&self) -> SymbolType {
        match self {
            Wf::F | Wf::P | Wf::A(_) | Wf::I(_) | Wf::B => SymbolType::NonTerminal,
            _ => SymbolType::Terminal,
        }
    }
}

fn generate(age: u32, p: &WildflowerParams, season: f32, dormancy_offset: f32) -> Vec<Action> {
    let age =
        (age as f32 * (1.0 - super::dormancy_factor(season, dormancy_offset, 0.65))).round() as u32;
    use Wf::*;
    const GOLDEN_ANGLE: f32 = 2.399655;

    // Flowering phenology: blooms spring → early summer; seed heads persist into autumn.
    let show_flower = season > 0.05 && season < 0.50;
    let show_seed = !show_flower && season < 0.72;
    let seed_colour = glam::vec3(0.62, 0.48, 0.18);
    let effective_flower_colour = if show_flower {
        p.flower_colour
    } else {
        seed_colour
    };
    let seed_size = p.flower_size * 0.55; // seed heads are smaller than flowers

    let branch_angle = p.branch_angle_deg.to_radians();
    let branch_len = p.branch_len;
    let branch_radius = p.branch_radius;
    let flower_size = p.flower_size;
    let leaf_size = p.leaf_size;
    let leaf_colour = p.leaf_colour;
    let stem_colour = p.stem_colour;

    // Flower/seed suffix is baked at construction time from season state (not symbol params).
    let flower_suffix: Vec<Wf> = if show_flower {
        vec![
            Push,
            Colour(effective_flower_colour),
            Leaf(flower_size, flower_size),
            Pop,
        ]
    } else if show_seed {
        vec![
            Push,
            Colour(effective_flower_colour),
            Leaf(seed_size, seed_size),
            Pop,
        ]
    } else {
        vec![]
    };

    let rule_p = Rule::Normal(P, &[Push, A(rng::random_range(0.0, 10.0f32)), Pop]);

    let rule_a = Rule::Parametric(A(0.0), &move |s: &Wf, out: &mut Vec<Wf>| {
        if let &Wf::A(a) = s {
            out.extend([
                F,
                F,
                F,
                I(40.0),
                Turn(rng::random_range(-a, a).to_radians()),
                Roll(GOLDEN_ANGLE),
                A(a + 1.0),
            ]);
            out.extend_from_slice(&flower_suffix);
            true
        } else {
            false
        }
    });

    let rule_i = Rule::Parametric(I(0.0), &move |s: &Wf, out: &mut Vec<Wf>| {
        if let &Wf::I(a) = s {
            out.extend([
                Push,
                Turn(-a.to_radians()),
                B,
                Pop,
                Roll(GOLDEN_ANGLE),
                Push,
                Turn(a.to_radians()),
                B,
                Pop,
            ]);
            true
        } else {
            false
        }
    });

    let rule_b_grow = Rule::Stochastic(B, 0.5, &[
        F,
        B,
        Push,
        Colour(leaf_colour),
        Leaf(leaf_size, leaf_size),
        Pop,
    ]);
    let rule_b_stop = Rule::Normal(B, &[I(rng::random_range(0.0, branch_angle.to_degrees()))]);

    let mut lsystem: LSystem<Wf> = LSystem::new(&[Colour(stem_colour), P], vec![
        rule_p,
        rule_a,
        rule_i,
        rule_b_grow,
        rule_b_stop,
    ]);
    lsystem.evolve(age as usize);

    lsystem
        .current()
        .iter()
        .map(|&s| match s {
            F => Action::Branch(branch_len, branch_radius),
            Turn(a) => Action::Turn(a),
            Roll(a) => Action::Roll(a),
            Colour(c) => Action::Colour(c),
            Leaf(w, h) => Action::Leaf(w, h),
            Push => Action::Push,
            Pop => Action::Pop,
            _ => Action::Nop,
        })
        .collect()
}
