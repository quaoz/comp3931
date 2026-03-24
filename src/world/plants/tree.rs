//! Tree plant with stochastic branching for natural variation.

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
    world::plants::{Species, lerp_bark_colour},
};

#[derive(Clone)]
pub struct TreeParams {
    pub max_iterations: u32,
    pub leaf_colour: Vec3,
    pub young_bark: Vec3,
    pub old_bark: Vec3,
    pub branch_angle_deg: f32,
    pub f_init: f32,
    pub f_max: f32,
    pub f_rand: f32,
    pub f_growth: f32,
    pub branch_radius_ratio: f32,
    pub min_branch_radius: f32,
    pub leaf_width: f32,
    pub leaf_height: f32,
    pub angle_jitter: f32,
    pub shed_age: u32,
}

impl Default for TreeParams {
    fn default() -> Self {
        Self {
            max_iterations: 8,
            leaf_colour: vec3(0.22, 0.65, 0.18),
            young_bark: vec3(0.55, 0.38, 0.20),
            old_bark: vec3(0.30, 0.22, 0.14),
            branch_angle_deg: 30.7,
            f_init: 0.05,
            f_max: 0.25,
            f_rand: 0.02,
            f_growth: 1.35,
            branch_radius_ratio: 0.28,
            min_branch_radius: 0.008,
            leaf_width: 0.09,
            leaf_height: 0.14,
            angle_jitter: 0.12,
            shed_age: 7,
        }
    }
}

pub struct Tree;

impl Species for Tree {
    type Params = TreeParams;

    const TYPE: PlantType = PlantType::Tree;

    fn generate(age: u32, p: &TreeParams, season: f32, dormancy_offset: f32) -> Vec<Action> {
        generate(age, p, season, dormancy_offset)
    }

    fn max_iterations(p: &TreeParams) -> u32 {
        p.max_iterations
    }

    fn colour(p: &TreeParams, iteration: u32) -> Vec3 {
        lerp_bark_colour(iteration, p.max_iterations as f32, p.young_bark, p.old_bark)
    }

    fn ui(p: &mut TreeParams, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("tree_params")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                changed |= widget::row(
                    ui,
                    "Max iterations",
                    egui::DragValue::new(&mut p.max_iterations).range(1..=15),
                );
                changed |= widget::row(
                    ui,
                    "Branch angle",
                    egui::Slider::new(&mut p.branch_angle_deg, 5.0..=60.0).suffix("°"),
                );
                changed |= widget::row(
                    ui,
                    "Angle jitter",
                    egui::Slider::new(&mut p.angle_jitter, 0.0..=0.5).max_decimals(3),
                );
                changed |= widget::row(
                    ui,
                    "Init seg length",
                    egui::Slider::new(&mut p.f_init, 0.01..=0.15).max_decimals(3),
                );
                changed |= widget::row(
                    ui,
                    "Seg growth rate",
                    egui::Slider::new(&mut p.f_growth, 1.0..=2.0).max_decimals(2),
                );
                changed |= widget::row(
                    ui,
                    "Max seg length",
                    egui::Slider::new(&mut p.f_max, 0.05..=0.6).max_decimals(3),
                );
                changed |= widget::row(
                    ui,
                    "Length jitter",
                    egui::Slider::new(&mut p.f_rand, 0.0..=0.1).max_decimals(3),
                );
                changed |= widget::row(
                    ui,
                    "Radius ratio",
                    egui::Slider::new(&mut p.branch_radius_ratio, 0.05..=0.6).max_decimals(2),
                );
                changed |= widget::row(
                    ui,
                    "Min branch radius",
                    egui::Slider::new(&mut p.min_branch_radius, 0.001..=0.05).max_decimals(4),
                );
                changed |= widget::row(
                    ui,
                    "Leaf width",
                    egui::Slider::new(&mut p.leaf_width, 0.01..=0.3).max_decimals(3),
                );
                changed |= widget::row(
                    ui,
                    "Leaf height",
                    egui::Slider::new(&mut p.leaf_height, 0.01..=0.4).max_decimals(3),
                );
                changed |= widget::row(ui, "Shed age", egui::Slider::new(&mut p.shed_age, 1..=15));
                changed |= widget::colour_row(ui, "Leaf colour", &mut p.leaf_colour);
                changed |= widget::colour_row(ui, "Young bark", &mut p.young_bark);
                changed |= widget::colour_row(ui, "Old bark", &mut p.old_bark);
            });

        if ui.button("Reset").clicked() {
            *p = TreeParams::default();
            changed = true;
        }

        changed
    }
}

// ── Symbol enum ──

#[derive(Debug, Copy, Clone)]
enum Ts {
    T,
    X,
    G,
    F(f32, f32), // (length, diameter)
    L,
    Roll(f32),
    Rl,
    Rr,
    Tl,
    Tr,
    Pu,
    Pd,
    Push,
    Pop,
    Cut,
    SetColour(Vec3),
}

impl PartialEq for Ts {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Symbol for Ts {
    fn symbol_type(&self) -> SymbolType {
        match self {
            Ts::T | Ts::X | Ts::G | Ts::F(..) => SymbolType::NonTerminal,
            _ => SymbolType::Terminal,
        }
    }
}

impl Display for Ts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::T => write!(f, "T"),
            Self::X => write!(f, "X"),
            Self::G => write!(f, "G"),
            Self::F(l, d) => write!(f, "F({l}, {d})"),
            Self::L => write!(f, "L"),
            Self::Roll(angle) => fmt_angle(f, '/', '\\', *angle),
            Self::Rl => write!(f, "Rl"),
            Self::Rr => write!(f, "Rr"),
            Self::Tl => write!(f, "Tl"),
            Self::Tr => write!(f, "Tr"),
            Self::Pu => write!(f, "Pu"),
            Self::Pd => write!(f, "Pd"),
            Self::Push => write!(f, "["),
            Self::Pop => write!(f, "]"),
            Self::Cut => write!(f, "!"),
            Self::SetColour(c) => write!(f, "C({})", c),
        }
    }
}

fn generate(age: u32, p: &TreeParams, season: f32, dormancy_offset: f32) -> Vec<Action> {
    use Ts::*;

    // Deciduous leaf shedding: ramps up through autumn, peaks in winter, recovers
    // in spring. Peaks at 0.85 — a few leaves persist, so the crown is never bare.
    let shed_prob = super::dormancy_factor(season, dormancy_offset, 0.85) as f64;

    let bark = lerp_bark_colour(age, p.max_iterations as f32, p.young_bark, p.old_bark);

    // Autumn colouration: leaves turn amber-gold before shedding.
    let leaf_colour =
        super::autumn_colour(season, p.leaf_colour, vec3(0.82, 0.52, 0.08), 0.48, 0.78);

    // Spring elongation: new shoots grow faster in early spring.
    let spring = super::spring_surge(season);

    let shedding = age >= p.shed_age;
    let branch_angle = p.branch_angle_deg.to_radians();
    let f_growth = p.f_growth * (1.0 + 0.20 * spring);
    let d_init = p.f_init * p.branch_radius_ratio;
    let d_max = p.f_max * p.branch_radius_ratio;

    let green = SetColour(leaf_colour);
    let brown = SetColour(bark);

    let rule_f = Rule::Parametric(F(0.0, 0.0), &|s: &Ts, out: &mut Vec<Ts>| {
        if let &Ts::F(l, d) = s {
            let r = rng::random_range(-p.f_rand, p.f_rand);
            out.push(F(
                (l * f_growth + r).min(p.f_max),
                (d * f_growth).min(d_max),
            ));
            true
        } else {
            false
        }
    });

    let leaf = |orients: &[Ts], shed: bool| -> Vec<Ts> {
        let mut v = vec![green, Push];
        v.extend_from_slice(orients);
        v.push(if shed { Cut } else { L });
        v.extend([Pop, brown]);
        v
    };
    let mut rule_g_out = leaf(&[Rl, Rl, Tr, Tr], shedding && rng::random_bool(shed_prob));
    rule_g_out.extend(leaf(
        &[Rl, Rl, Tr, Tr, Tr],
        shedding && rng::random_bool(shed_prob),
    ));
    rule_g_out.extend(leaf(
        &[Rr, Pu, Tl, Tl],
        shedding && rng::random_bool(shed_prob),
    ));
    rule_g_out.extend(leaf(
        &[Rr, Rr, Pd, Pd, Tl, Tl],
        shedding && rng::random_bool(shed_prob),
    ));
    let rule_g = Rule::Normal(G, &rule_g_out);

    let jt = Roll(rng::random_range(-p.angle_jitter, p.angle_jitter));
    let rule_t = Rule::Normal(T, &[
        F(p.f_init, d_init),
        F(p.f_init, d_init),
        F(p.f_init, d_init),
        jt,
        Push,
        Rl,
        Tl,
        F(p.f_init, d_init),
        X,
        Push,
        Tr,
        Tr,
        G,
        Pop,
        Pop,
        Push,
        Rl,
        Rl,
        Rl,
        Rl,
        Tl,
        F(p.f_init, d_init),
        X,
        Push,
        Tl,
        Tl,
        G,
        Pop,
        Pop,
        Rl,
        Rl,
        Rl,
        Rl,
        Rl,
        Rl,
        Rl,
        Rl,
        Rl,
        Tl,
        F(p.f_init, d_init),
        X,
        Push,
        Tr,
        G,
        Pop,
    ]);

    let jx = Roll(rng::random_range(-p.angle_jitter, p.angle_jitter));
    let rule_x = Rule::Normal(X, &[
        F(p.f_init, d_init),
        jx,
        Push,
        Rl,
        Tl,
        F(p.f_init, d_init),
        X,
        Push,
        Tr,
        Tr,
        G,
        Pop,
        Pop,
        Push,
        Rl,
        Rl,
        Rl,
        Rl,
        Tl,
        F(p.f_init, d_init),
        X,
        Push,
        Tl,
        Tl,
        G,
        Pop,
        Pop,
        Rl,
        Rl,
        Rl,
        Rl,
        Rl,
        Rl,
        Rl,
        Rl,
        Rl,
        Tl,
        F(p.f_init, d_init),
        X,
        Push,
        Tr,
        G,
        Pop,
    ]);

    let mut lsystem = LSystem::new(&[T], vec![rule_f, rule_g, rule_t, rule_x]);
    lsystem.evolve(age as usize);

    let leaf_width = p.leaf_width;
    let leaf_height = p.leaf_height;

    let mut actions = vec![Action::Colour(bark)];
    actions.extend(lsystem.current().iter().map(|&s| match s {
        F(l, d) => Action::Branch(l, d.max(p.min_branch_radius)),
        L => Action::Leaf(leaf_width, leaf_height),
        Roll(r) => Action::Roll(r),
        Rl => Action::Roll(-branch_angle),
        Rr => Action::Roll(branch_angle),
        Tl => Action::Turn(-branch_angle),
        Tr => Action::Turn(branch_angle),
        Pu => Action::Pitch(branch_angle),
        Pd => Action::Pitch(-branch_angle),
        Push => Action::Push,
        Pop => Action::Pop,
        Cut => Action::Cut,
        SetColour(c) => Action::Colour(c),
        _ => Action::Nop,
    }));
    actions
}
