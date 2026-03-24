//! Capsella (Shepherd's Purse) inspired by "The Algorithmic Beauty of Plants" fig 3.5, pg 74.

use std::fmt::Display;

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
pub struct CapsellaParams {
    pub stem_colour: Vec3,
    pub flower_colour: Vec3,
    pub pod_colour: Vec3,
    pub branch_radius: f32,
    pub leaf_width: f32,
    pub leaf_height: f32,
    pub flower_size: f32,
    pub pod_size: f32,
    pub branch_angle_deg: f32,
    pub leaf_angle_deg: f32,
    pub n_vegetative: u32,
    pub i_init: f32,
    pub i_max: f32,
    pub i_growth: f32,
    pub max_iterations: u32,
}

impl Default for CapsellaParams {
    fn default() -> Self {
        Self {
            stem_colour: vec3(0.3, 0.65, 0.2),
            flower_colour: vec3(0.95, 0.95, 0.95),
            pod_colour: vec3(0.1, 0.05, 0.03),
            branch_radius: 0.018,
            leaf_width: 0.12,
            leaf_height: 0.21,
            flower_size: 0.1,
            pod_size: 0.06,
            branch_angle_deg: 18.0,
            leaf_angle_deg: 70.0,
            n_vegetative: 15,
            i_init: 0.06,
            i_max: 0.1,
            i_growth: 1.15,
            max_iterations: 45,
        }
    }
}

pub struct Capsella;

impl Species for Capsella {
    type Params = CapsellaParams;

    const TYPE: PlantType = PlantType::Capsella;

    fn generate(age: u32, p: &CapsellaParams, season: f32, dormancy_offset: f32) -> Vec<Action> {
        generate(age, p, season, dormancy_offset)
    }

    fn max_iterations(p: &CapsellaParams) -> u32 {
        p.max_iterations
    }

    fn colour(p: &CapsellaParams, _iteration: u32) -> Vec3 {
        p.stem_colour
    }

    fn ui(p: &mut CapsellaParams, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("capsella_params")
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
                    "Vegetative steps",
                    egui::Slider::new(&mut p.n_vegetative, 1..=20),
                );
                changed |= widget::row(
                    ui,
                    "Branch angle",
                    egui::Slider::new(&mut p.branch_angle_deg, 5.0..=60.0).suffix("°"),
                );
                changed |= widget::row(
                    ui,
                    "Leaf angle",
                    egui::Slider::new(&mut p.leaf_angle_deg, 5.0..=85.0).suffix("°"),
                );
                changed |= widget::row(
                    ui,
                    "Internode length",
                    egui::Slider::new(&mut p.i_init, 0.01..=0.2).max_decimals(3),
                );
                changed |= widget::row(
                    ui,
                    "Branch radius",
                    egui::Slider::new(&mut p.branch_radius, 0.002..=0.05).max_decimals(4),
                );
                changed |= widget::row(
                    ui,
                    "Internode max",
                    egui::Slider::new(&mut p.i_max, 0.05..=0.4).max_decimals(3),
                );
                changed |= widget::row(
                    ui,
                    "Internode growth",
                    egui::Slider::new(&mut p.i_growth, 1.0..=1.5).max_decimals(2),
                );
                changed |= widget::row(
                    ui,
                    "Flower size",
                    egui::Slider::new(&mut p.flower_size, 0.005..=0.12).max_decimals(3),
                );
                changed |= widget::row(
                    ui,
                    "Pod size",
                    egui::Slider::new(&mut p.pod_size, 0.005..=0.12).max_decimals(3),
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
                changed |= widget::colour_row(ui, "Pod colour", &mut p.pod_colour);
            });

        if ui.button("Reset").clicked() {
            *p = CapsellaParams::default();
            changed = true;
        }

        changed
    }
}

#[derive(Debug, Copy, Clone)]
enum Cs {
    /// Vegetative apex: counts down from `n_vegetative`; at 0 transitions to `A`.
    SmallA(u8),
    /// Flowering apex: each step emits one lateral branch and continues.
    A,
    /// Angle accumulator: each step prepends `Pitch(-U_STEP)` and decrements.
    /// Implements ABOP's `u(t)` that progressively droops each lateral branch.
    U(u8),
    I(f32),
    /// Fruit pod countdown: counts down from `X_DELAY` to 0; pod visible once mature.
    X(u8),
    L,
    K,
    Pitch(f32),
    Roll(f32),
    Push,
    Pop,
}

impl PartialEq for Cs {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Symbol for Cs {
    fn symbol_type(&self) -> SymbolType {
        match self {
            Cs::SmallA(_) | Cs::A | Cs::U(_) | Cs::I(_) | Cs::X(_) => SymbolType::NonTerminal,
            _ => SymbolType::Terminal,
        }
    }
}

impl Display for Cs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SmallA(t) => write!(f, "a({t})"),
            Self::A => write!(f, "A"),
            Self::U(t) => write!(f, "u({t})"),
            Self::I(l) => write!(f, "I({l:.3})"),
            Self::X(t) => write!(f, "X({t})"),
            Self::L => write!(f, "L"),
            Self::K => write!(f, "K"),
            Self::Pitch(a) => fmt_angle(f, '^', '&', *a),
            Self::Roll(a) => fmt_angle(f, '/', '\\', *a),
            Self::Push => write!(f, "["),
            Self::Pop => write!(f, "]"),
        }
    }
}

fn generate(age: u32, p: &CapsellaParams, season: f32, dormancy_offset: f32) -> Vec<Action> {
    use std::f32::consts::PI;

    use Cs::*;

    // Spring annual: germinates and grows through spring, sets seed by summer.
    let age =
        (age as f32 * (1.0 - super::dormancy_factor(season, dormancy_offset, 0.65))).round() as u32;

    // 137.5° golden-angle divergence (ABOP: /(137.5) between successive nodes).
    const GOLDEN_ANGLE: f32 = 2.399655;
    // 9° per accumulator step (ABOP: &(9) in u(t) productions p6/p7).
    const U_STEP: f32 = PI / 20.0;
    // Number of u(t) accumulator steps; total extra droop = (U_STEPS+1) × 9° = 45°.
    const U_STEPS: u8 = 4;
    // Steps for fruit pod to reach full size.
    const X_DELAY: u8 = 5;
    // Season at which petals are replaced by pods.
    const BLOOM_END: f32 = 0.35;

    let leaf_angle = p.leaf_angle_deg.to_radians();
    let branch_angle = p.branch_angle_deg.to_radians();

    // K appearance: white petals in bloom window, dried pod colour thereafter.
    let k_colour = if season < BLOOM_END {
        p.flower_colour
    } else {
        p.pod_colour
    };
    // Pods disperse by mid-summer.
    let show_k = season < 0.65;

    // P1: vegetative apex counting down — leaf at leaf_angle, golden roll, internode, continue.
    let rule_small_a_tick = Rule::Parametric(SmallA(0), &move |s: &Cs, out: &mut Vec<Cs>| {
        if let &Cs::SmallA(t) = s
            && t > 0
        {
            out.extend([
                Push,
                Pitch(-leaf_angle),
                L,
                Pop,
                Roll(GOLDEN_ANGLE),
                I(p.i_init),
                SmallA(t - 1),
            ]);
            true
        } else {
            false
        }
    });

    // P2: vegetative → flowering transition (t == 0).
    let rule_small_a_flower = Rule::Parametric(SmallA(0), &move |s: &Cs, out: &mut Vec<Cs>| {
        if let &Cs::SmallA(t) = s
            && t == 0
        {
            out.extend([
                Push,
                Pitch(-leaf_angle),
                L,
                Pop,
                Roll(GOLDEN_ANGLE),
                I(p.i_init),
                A,
            ]);
            true
        } else {
            false
        }
    });

    // P3: flowering apex — lateral branch with angle accumulator and four petals/pods,
    // then golden-angle roll, internode, and continuation apex.
    // Mirrors ABOP: [&(18) u(4) FF I(10) I(5) X(5) KKKK] /(137.5) I(8) A
    let rule_a = Rule::Normal(A, &[
        Push,
        Pitch(-branch_angle),
        U(U_STEPS),
        I(p.i_init),
        I(p.i_init * 0.5),
        X(X_DELAY),
        K,
        K,
        K,
        K,
        Pop,
        Roll(GOLDEN_ANGLE),
        I(p.i_init),
        A,
    ]);

    // P4: internode elongation — length grows each step up to i_max.
    let rule_i = Rule::Parametric(I(0.0), &move |s: &Cs, out: &mut Vec<Cs>| {
        if let &Cs::I(l) = s {
            out.push(I((l * p.i_growth).min(p.i_max)));
            true
        } else {
            false
        }
    });

    // P5/P6: angle accumulator — each step prepends one &(U_STEP) and decrements t;
    // at t=0 the final Pitch is emitted and u disappears.
    // After U_STEPS+1 iterations the branch droops by branch_angle + (U_STEPS+1)*U_STEP.
    let rule_u_tick = Rule::Parametric(U(0), &move |s: &Cs, out: &mut Vec<Cs>| {
        if let &Cs::U(t) = s
            && t > 0
        {
            out.extend([Pitch(-U_STEP), U(t - 1)]);
            true
        } else {
            false
        }
    });

    let rule_u_done = Rule::Parametric(U(0), &move |s: &Cs, out: &mut Vec<Cs>| {
        if let &Cs::U(t) = s
            && t == 0
        {
            out.push(Pitch(-U_STEP));
            true
        } else {
            false
        }
    });

    // P7: fruit countdown — counts down to 0, after which the pod renders at full size.
    let rule_x = Rule::Parametric(X(0), &move |s: &Cs, out: &mut Vec<Cs>| {
        if let &Cs::X(t) = s
            && t > 0
        {
            out.push(X(t - 1));
            true
        } else {
            false
        }
    });

    let n_veg = p.n_vegetative.min(255) as u8;
    let mut lsystem: LSystem<Cs> = LSystem::new(&[I(p.i_init), SmallA(n_veg)], vec![
        rule_small_a_tick,
        rule_small_a_flower,
        rule_i,
        rule_u_tick,
        rule_u_done,
        rule_x,
        rule_a,
    ]);
    lsystem.evolve(age as usize);

    lsystem
        .current()
        .iter()
        .flat_map(|&s| match s {
            I(l) => vec![
                Action::Colour(p.stem_colour),
                Action::Branch(l, p.branch_radius),
            ],
            L => vec![
                Action::Colour(p.stem_colour),
                Action::Leaf(p.leaf_width, p.leaf_height),
            ],
            K if show_k => vec![
                Action::Colour(k_colour),
                Action::Leaf(p.flower_size, p.flower_size),
                Action::Colour(p.stem_colour),
            ],
            X(t) => {
                let maturity = X_DELAY.saturating_sub(t) as f32 / X_DELAY as f32;
                let size = p.pod_size * maturity;
                if size > 0.001 {
                    vec![
                        Action::Colour(p.pod_colour),
                        Action::Leaf(size * 0.75, size * 1.3),
                        Action::Colour(p.stem_colour),
                    ]
                } else {
                    vec![]
                }
            }
            Pitch(a) => vec![Action::Pitch(a)],
            Roll(a) => vec![Action::Roll(a)],
            Push => vec![Action::Push],
            Pop => vec![Action::Pop],
            _ => vec![],
        })
        .collect()
}
