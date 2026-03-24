//! Lychnis (Campion) inspired by "The Algorithmic Beauty of Plants" fig 3.14, pg 84.

use std::{
    f32::consts::{FRAC_PI_2, FRAC_PI_3},
    fmt::Display,
};

use glam::{Vec3, vec3};

use crate::{
    settings::PlantType,
    util::{
        lsystem::{LSystem, Rule, Symbol, SymbolType, fmt_angle},
        turtle::Action,
        widget,
    },
    world::plants::Species,
};

#[derive(Clone)]
pub struct LychnisParams {
    pub stem_colour: Vec3,
    pub flower_colour: Vec3,
    pub leaf_colour: Vec3,
    pub branch_radius: f32,
    pub flower_size: f32,
    pub leaf_width: f32,
    pub leaf_height: f32,
    pub branch_angle_deg: f32,
    pub i_init: f32,
    pub max_iterations: u32,
}

impl Default for LychnisParams {
    fn default() -> Self {
        Self {
            stem_colour: vec3(0.35, 0.55, 0.25),
            flower_colour: vec3(0.9, 0.2, 0.4),
            leaf_colour: vec3(0.3, 0.65, 0.2),
            branch_radius: 0.025,
            flower_size: 0.1,
            leaf_width: 0.1,
            leaf_height: 0.135,
            branch_angle_deg: 35.0,
            i_init: 0.02,
            max_iterations: 30,
        }
    }
}

pub struct Lychnis;

impl Species for Lychnis {
    type Params = LychnisParams;

    const TYPE: PlantType = PlantType::Lychnis;

    fn generate(age: u32, p: &LychnisParams, season: f32, dormancy_offset: f32) -> Vec<Action> {
        generate(age, p, season, dormancy_offset)
    }

    fn max_iterations(p: &LychnisParams) -> u32 {
        p.max_iterations
    }

    fn colour(p: &LychnisParams, _iteration: u32) -> Vec3 {
        p.stem_colour
    }

    fn ui(p: &mut LychnisParams, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("lychnis_params")
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
                    egui::Slider::new(&mut p.branch_angle_deg, 5.0..=70.0).suffix("°"),
                );
                changed |= widget::row(
                    ui,
                    "Internode length",
                    egui::Slider::new(&mut p.i_init, 0.005..=0.1).max_decimals(4),
                );
                changed |= widget::row(
                    ui,
                    "Branch radius",
                    egui::Slider::new(&mut p.branch_radius, 0.005..=0.06).max_decimals(4),
                );
                changed |= widget::row(
                    ui,
                    "Flower size",
                    egui::Slider::new(&mut p.flower_size, 0.01..=0.15).max_decimals(3),
                );
                changed |= widget::row(
                    ui,
                    "Leaf width",
                    egui::Slider::new(&mut p.leaf_width, 0.01..=0.2).max_decimals(3),
                );
                changed |= widget::row(
                    ui,
                    "Leaf height",
                    egui::Slider::new(&mut p.leaf_height, 0.01..=0.3).max_decimals(3),
                );
                changed |= widget::colour_row(ui, "Stem colour", &mut p.stem_colour);
                changed |= widget::colour_row(ui, "Flower colour", &mut p.flower_colour);
                changed |= widget::colour_row(ui, "Leaf colour", &mut p.leaf_colour);
            });

        if ui.button("Reset").clicked() {
            *p = LychnisParams::default();
            changed = true;
        }

        changed
    }
}

#[derive(Debug, Copy, Clone)]
enum Ls {
    A(u8),
    I(u8),
    Branch,
    Leaf(u8),
    Flower(u8),
    Roll(f32),
    Pitch(f32),
    Push,
    Pop,
}

impl PartialEq for Ls {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Symbol for Ls {
    fn symbol_type(&self) -> SymbolType {
        match self {
            Ls::A(..) | Ls::I(..) | Ls::Leaf(..) | Ls::Flower(..) => SymbolType::NonTerminal,
            _ => SymbolType::Terminal,
        }
    }
}

impl Display for Ls {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::A(d) => write!(f, "A({d})"),
            Self::I(l) => write!(f, "I({l})"),
            Self::Branch => write!(f, "B"),
            Self::Leaf(a) => write!(f, "L({a})"),
            Self::Flower(a) => write!(f, "K({a})"),
            Self::Roll(angle) => fmt_angle(f, '/', '\\', *angle),
            Self::Pitch(angle) => fmt_angle(f, '^', '&', *angle),
            Self::Push => write!(f, "["),
            Self::Pop => write!(f, "]"),
        }
    }
}

fn generate(age: u32, p: &LychnisParams, season: f32, dormancy_offset: f32) -> Vec<Action> {
    use Ls::*;

    const DELAY: u8 = 7;
    const BLOOM_START: f32 = 0.10;
    const BLOOM_END: f32 = 0.48;
    const MAX_DORMANCY: f32 = 0.65;
    const LEAF_MAX_SIZE: u8 = 10;
    const FLOWER_MAX_SIZE: u8 = 10;

    let age = (age as f32 * (1.0 - super::dormancy_factor(season, dormancy_offset, MAX_DORMANCY)))
        .round() as u32;

    // A waiting phase: delay < 7, just increments. Head A(0) matches when d < 7.
    let rule_a_wait = Rule::Parametric(A(0), &|s: &Ls, out: &mut Vec<Ls>| {
        if let &Ls::A(d) = s
            && d < DELAY
        {
            out.push(A(d + 1));
            true
        } else {
            false
        }
    });

    // A expanded phase: delay >= 7, emit full structure. Head A(7) matches when d >= 7.
    let rule_a_expand = Rule::Parametric(A(0), &|s: &Ls, out: &mut Vec<Ls>| {
        if let &Ls::A(d) = s
            && d >= DELAY
        {
            out.extend([
                Branch,
                I(20),
                Push,
                Pitch(FRAC_PI_3),
                Leaf(0),
                Pop,
                Roll(FRAC_PI_2),
                Push,
                Pitch(FRAC_PI_2),
                A(0),
                Pop,
                Roll(FRAC_PI_2),
                Push,
                Pitch(FRAC_PI_3),
                Leaf(0),
                Pop,
                Roll(FRAC_PI_2),
                Push,
                Pitch(FRAC_PI_2),
                A(4),
                Pop,
                Branch,
                I(10),
            ]);

            if season > BLOOM_START && season < BLOOM_END {
                out.push(Flower(0));
            }
            true
        } else {
            false
        }
    });

    // I recursive: count > 0, expands. Head I(1) matches when c > 0.
    let rule_b = Rule::Parametric(I(0), &|s: &Ls, out: &mut Vec<Ls>| {
        if let &Ls::I(count) = s
            && count > 0
        {
            out.extend([Branch, Branch, I(count - 1)]);
            true
        } else {
            false
        }
    });

    let rule_c = Rule::Parametric(Leaf(0), &|s: &Ls, out: &mut Vec<Ls>| {
        if let &Ls::Leaf(size) = s
            && size < LEAF_MAX_SIZE
        {
            out.push(Leaf(size + 1));
            true
        } else {
            false
        }
    });

    let rule_d = Rule::Parametric(Flower(0), &|s: &Ls, out: &mut Vec<Ls>| {
        if let &Ls::Flower(size) = s
            && size < FLOWER_MAX_SIZE
        {
            out.push(Flower(size + 1));
            true
        } else {
            false
        }
    });

    let mut lsystem: LSystem<Ls> = LSystem::new(&[A(7)], vec![
        rule_a_wait,
        rule_a_expand,
        rule_b,
        rule_c,
        rule_d,
    ]);
    lsystem.evolve(age as usize);

    lsystem
        .current()
        .iter()
        .flat_map(|&s| {
            if let Flower(size) = s {
                let scale = size as f32 / FLOWER_MAX_SIZE as f32;
                vec![
                    Action::Push,
                    Action::Colour(p.flower_colour),
                    Action::Leaf(p.flower_size * scale, p.flower_size * scale),
                    Action::Colour(p.stem_colour),
                    Action::Pop,
                ]
            } else {
                vec![match s {
                    Branch => Action::Branch(p.i_init, p.branch_radius),
                    Leaf(size) => {
                        let scale = size as f32 / LEAF_MAX_SIZE as f32;
                        Action::Leaf(p.leaf_width * scale, p.leaf_height * scale)
                    }
                    Roll(angle) => Action::Roll(angle),
                    Pitch(angle) => Action::Pitch(angle),
                    Push => Action::Push,
                    Pop => Action::Pop,
                    _ => Action::Nop,
                }]
            }
        })
        .collect()
}
