//! Mycelis (Wall Lettuce) inspired by "The Algorithmic Beauty of Plants" fig 3.18-3.19, pg 90-91.

use std::{
    f32::consts::{FRAC_PI_6, FRAC_PI_8, PI},
    fmt::Display,
};

use glam::{Vec3, vec3};

use crate::{
    settings::PlantType,
    util::{
        lsystem::{ContextAction, LSystem, Rule, Symbol, SymbolType, fmt_angle},
        rng,
        turtle::Action,
        widget,
    },
    world::plants::Species,
};

#[derive(Clone)]
pub struct MycelisParams {
    pub stem_colour: Vec3,
    pub flower_colour: Vec3,
    pub branch_radius: f32,
    pub flower_size: f32,
    pub branch_angle_deg: f32,
    pub i_init: f32,
    pub max_iterations: u32,
}

impl Default for MycelisParams {
    fn default() -> Self {
        Self {
            stem_colour: vec3(0.4, 0.6, 0.2),
            flower_colour: vec3(0.9, 0.85, 0.1),
            branch_radius: 0.005,
            flower_size: 0.04,
            branch_angle_deg: 30.0,
            i_init: 0.10,
            max_iterations: 1000,
        }
    }
}

pub struct Mycelis;

impl Species for Mycelis {
    type Params = MycelisParams;

    const TYPE: PlantType = PlantType::Mycelis;

    fn generate(age: u32, p: &MycelisParams, season: f32, dormancy_offset: f32) -> Vec<Action> {
        generate(age, p, season, dormancy_offset)
    }

    fn max_iterations(p: &MycelisParams) -> u32 {
        p.max_iterations
    }

    fn colour(p: &MycelisParams, _iteration: u32) -> Vec3 {
        p.stem_colour
    }

    fn ui(p: &mut MycelisParams, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("mycelis_params")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                changed |= widget::row(
                    ui,
                    "Max iterations",
                    egui::DragValue::new(&mut p.max_iterations).range(1..=100),
                );
                changed |= widget::row(
                    ui,
                    "Branch angle",
                    egui::Slider::new(&mut p.branch_angle_deg, 5.0..=60.0).suffix("\u{b0}"),
                );
                changed |= widget::row(
                    ui,
                    "Internode length",
                    egui::Slider::new(&mut p.i_init, 0.02..=0.3).max_decimals(3),
                );
                changed |= widget::row(
                    ui,
                    "Branch radius",
                    egui::Slider::new(&mut p.branch_radius, 0.001..=0.03).max_decimals(4),
                );
                changed |= widget::row(
                    ui,
                    "Flower size",
                    egui::Slider::new(&mut p.flower_size, 0.005..=0.12).max_decimals(3),
                );
                changed |= widget::colour_row(ui, "Stem colour", &mut p.stem_colour);
                changed |= widget::colour_row(ui, "Flower colour", &mut p.flower_colour);
            });

        if ui.button("Reset").clicked() {
            *p = MycelisParams::default();
            changed = true;
        }

        changed
    }
}

#[derive(Debug, Copy, Clone)]
enum Mys {
    A(u8),
    I(u8),
    K(u8),
    T,
    V,
    M,
    G,
    S,
    W,
    F,
    Leaf,
    Turn(f32),
    Roll(f32),
    Colour(Vec3),
    Push,
    Pop,
}

impl PartialEq for Mys {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Symbol for Mys {
    fn symbol_type(&self) -> SymbolType {
        match self {
            Mys::A(_) | Mys::I(_) | Mys::K(_) | Mys::M | Mys::S | Mys::G | Mys::T | Mys::W => {
                SymbolType::NonTerminal
            }
            _ => SymbolType::Terminal,
        }
    }

    fn context(&self) -> ContextAction {
        match self {
            Mys::Push => ContextAction::BranchStart,
            Mys::Pop => ContextAction::BranchEnd,
            Mys::M | Mys::S | Mys::T | Mys::V | Mys::W => ContextAction::Consider,
            _ => ContextAction::Ignore,
        }
    }
}

impl Display for Mys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::A(t) => write!(f, "A({t})"),
            Self::I(t) => write!(f, "I({t})"),
            Self::K(c) => write!(f, "K{c}"),
            Self::T => write!(f, "T"),
            Self::V => write!(f, "V"),
            Self::M => write!(f, "M"),
            Self::G => write!(f, "G"),
            Self::S => write!(f, "S"),
            Self::W => write!(f, "W"),
            Self::F => write!(f, "F"),
            Self::Leaf => write!(f, "L"),
            Self::Turn(angle) => fmt_angle(f, '+', '-', *angle),
            Self::Roll(angle) => fmt_angle(f, '/', '\\', *angle),
            Self::Colour(c) => write!(f, "C({})", c),
            Self::Push => write!(f, "["),
            Self::Pop => write!(f, "]"),
        }
    }
}

fn generate(age: u32, p: &MycelisParams, season: f32, dormancy_offset: f32) -> Vec<Action> {
    const MAX_DORMANCY: f32 = 0.65;
    let age = (age as f32 * (1.0 - super::dormancy_factor(season, dormancy_offset, MAX_DORMANCY)))
        .round() as u32;
    use Mys::*;

    const GREEN: Vec3 = vec3(0.07, 0.1, 0.07);
    const PURPLE: Vec3 = vec3(0.15, 0.14, 0.19);
    const FLOWER_MAX: u8 = 7;

    let rule_1 = Rule::ContextSensitive(A(0), Some(S), None, &[
        T,
        V,
        Colour(p.flower_colour),
        K(FLOWER_MAX),
        Colour(GREEN),
    ]);
    let rule_2 = Rule::ContextSensitive(A(0), Some(V), None, &[
        T,
        V,
        Colour(p.flower_colour),
        K(FLOWER_MAX),
        Colour(GREEN),
    ]);
    let rule_3 = Rule::Parametric(A(0), &|s, out| {
        if let A(t) = s
            && *t > 0
        {
            out.push(A(t - 1));
            true
        } else {
            false
        }
    });
    let rule_4 = Rule::Parametric(A(0), &|s, out| {
        if let A(t) = s
            && *t == 0
        {
            out.extend([
                M,
                Push,
                Turn(FRAC_PI_8 + p.branch_angle_deg.to_radians()),
                Leaf,
                Turn(-FRAC_PI_8),
                G,
                Pop,
                F,
                Roll(rng::random_range(FRAC_PI_6, PI)),
                A(2),
            ]);
            true
        } else {
            false
        }
    });
    let rule_5 = Rule::ContextSensitive(M, Some(S), None, &[S]);
    let rule_6 = Rule::ContextSensitive(S, None, Some(T), &[T]);
    let rule_7 = Rule::ContextSensitive(G, Some(T), None, &[F, A(2)]);
    let rule_8 = Rule::ContextSensitive(M, Some(V), None, &[S]);
    let rule_9 = Rule::ContextSensitive(T, None, Some(V), &[W]);
    let rule_10 = Rule::Normal(W, &[V]);
    let rule_11 = Rule::Parametric(I(0), &|s, out| {
        if let I(t) = s
            && *t > 0
        {
            out.push(I(t - 1));
            true
        } else {
            false
        }
    });
    let rule_12 = Rule::Parametric(I(0), &|s, out| {
        if let I(t) = s
            && *t == 0
        {
            out.push(S);
            true
        } else {
            false
        }
    });
    let rule_13 = Rule::Parametric(K(0), &|s, out| {
        if let K(c) = s
            && *c > 0
        {
            out.push(K(c - 1));
            true
        } else {
            false
        }
    });

    let mut lsystem: LSystem<Mys> = LSystem::new(&[I(20), F, A(0)], vec![
        rule_1, rule_2, rule_3, rule_4, rule_5, rule_6, rule_7, rule_8, rule_9, rule_10, rule_11,
        rule_12, rule_13,
    ]);
    lsystem.evolve(age as usize);

    lsystem
        .current()
        .iter()
        .flat_map(|&s| {
            if let K(c) = s {
                let (scale, colour) = match c {
                    6 | 5 => (1.0, p.flower_colour),
                    4 | 3 => (0.8, p.flower_colour),
                    2 | 1 => (0.8, vec3(0.8, 0.3, 0.1)),
                    0 => (0.5, vec3(0.4, 0.2, 0.05)),
                    _ => (0.5, p.flower_colour),
                };

                vec![
                    Action::Colour(colour),
                    Action::Leaf(scale * p.flower_size, scale * p.flower_size),
                    Action::Colour(GREEN),
                ]
            } else if Leaf == s {
                vec![
                    Action::Colour(p.stem_colour),
                    Action::Leaf(0.02, 0.08),
                    Action::Colour(p.stem_colour),
                ]
            } else {
                vec![match s {
                    G | F => Action::Branch(p.i_init, p.branch_radius),
                    M => Action::Colour(p.stem_colour),
                    S => Action::Colour(PURPLE),
                    T | W | V => Action::Colour(GREEN),
                    Turn(a) => Action::Turn(a),
                    Roll(a) => Action::Roll(a),
                    Colour(c) => Action::Colour(c),
                    Push => Action::Push,
                    Pop => Action::Pop,
                    _ => Action::Nop,
                }]
            }
        })
        .collect()
}
