//! Wildflower — stochastic flowering plant with golden-angle phyllotaxis.

use std::fmt::Display;

use glam::{Vec3, vec3};

use crate::{
    settings::PlantType,
    util::{
        lsystem::{LSystem, Rule, Symbol, SymbolType, fmt_angle},
        rng,
        turtle::Action,
        widget,
    },
    world::plants::Species,
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

pub struct Wildflower;

impl Species for Wildflower {
    type Params = WildflowerParams;

    const TYPE: PlantType = PlantType::Wildflower;

    fn generate(age: u32, p: &WildflowerParams, season: f32, dormancy_offset: f32) -> Vec<Action> {
        generate(age, p, season, dormancy_offset)
    }

    fn max_iterations(p: &WildflowerParams) -> u32 {
        p.max_iterations
    }

    fn colour(p: &WildflowerParams, _iteration: u32) -> Vec3 {
        p.stem_colour
    }

    fn ui(p: &mut WildflowerParams, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("wildflower_params")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                changed |= widget::row(
                    ui,
                    "Max iterations",
                    egui::DragValue::new(&mut p.max_iterations).range(1..=30),
                );
                changed |= widget::row(
                    ui,
                    "Branch angle",
                    egui::Slider::new(&mut p.branch_angle_deg, 5.0..=80.0).suffix("\u{b0}"),
                );
                changed |= widget::row(
                    ui,
                    "Branch length",
                    egui::Slider::new(&mut p.branch_len, 0.02..=0.3).max_decimals(3),
                );
                changed |= widget::row(
                    ui,
                    "Branch radius",
                    egui::Slider::new(&mut p.branch_radius, 0.002..=0.06).max_decimals(4),
                );
                changed |= widget::row(
                    ui,
                    "Flower size",
                    egui::Slider::new(&mut p.flower_size, 0.005..=0.12).max_decimals(3),
                );
                changed |= widget::row(
                    ui,
                    "Leaf size",
                    egui::Slider::new(&mut p.leaf_size, 0.005..=0.08).max_decimals(3),
                );
                changed |= widget::colour_row(ui, "Stem colour", &mut p.stem_colour);
                changed |= widget::colour_row(ui, "Flower colour", &mut p.flower_colour);
                changed |= widget::colour_row(ui, "Leaf colour", &mut p.leaf_colour);
            });

        if ui.button("Reset").clicked() {
            *p = WildflowerParams::default();
            changed = true;
        }

        changed
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
            Self::Turn(angle) => fmt_angle(f, '+', '-', *angle),
            Self::Roll(angle) => fmt_angle(f, '/', '\\', *angle),
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

    // Flower/seed suffix is baked at construction time from season state (not symbol params).
    let flower_suffix: Vec<Wf> = if show_flower {
        vec![
            Push,
            Colour(effective_flower_colour),
            Leaf(p.flower_size, p.flower_size),
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
        Colour(p.leaf_colour),
        Leaf(p.leaf_size, p.leaf_size),
        Pop,
    ]);
    let rule_b_stop = Rule::Normal(B, &[I(rng::random_range(0.0, branch_angle.to_degrees()))]);

    let mut lsystem: LSystem<Wf> = LSystem::new(&[Colour(p.stem_colour), P], vec![
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
            F => Action::Branch(p.branch_len, p.branch_radius),
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
