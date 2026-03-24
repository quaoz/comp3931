//! Carrot plant inspired by "The Algorithmic Beauty of Plants" fig 3.23, pg 96.

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
pub struct CarrotParams {
    pub stem_colour: Vec3,
    pub flower_colour: Vec3,
    pub leaf_colour: Vec3,
    pub branch_radius: f32,
    pub leaf_width: f32,
    pub leaf_height: f32,
    pub floret_size: f32,
    pub ray_angle_deg: f32,
    pub sub_angle_deg: f32,
    /// Lateral spread of each leaflet away from the rachis.
    pub leaf_angle_deg: f32,
    /// Angle of each compound leaf from vertical (rosette spread).
    pub rosette_angle_deg: f32,
    /// Upward arch added per rachis segment (degrees).
    pub rachis_arch_deg: f32,
    /// Number of pinnate segments per compound leaf.
    pub leaf_depth: u32,
    pub i_init: f32,
    pub i_max: f32,
    pub i_growth: f32,
    pub flower_delay: u32,
    pub max_iterations: u32,
}

impl Default for CarrotParams {
    fn default() -> Self {
        Self {
            stem_colour: vec3(0.4, 0.65, 0.2),
            flower_colour: vec3(0.95, 0.95, 0.95),
            leaf_colour: vec3(0.35, 0.75, 0.2),
            branch_radius: 0.022,
            leaf_width: 0.04,
            leaf_height: 0.07,
            floret_size: 0.025,
            ray_angle_deg: 40.0,
            sub_angle_deg: 30.0,
            leaf_angle_deg: 40.0,
            rosette_angle_deg: 55.0,
            rachis_arch_deg: 10.0,
            leaf_depth: 4,
            i_init: 0.05,
            i_max: 0.16,
            i_growth: 1.2,
            flower_delay: 3,
            max_iterations: 8,
        }
    }
}

pub struct Carrot;

impl Species for Carrot {
    type Params = CarrotParams;

    const TYPE: PlantType = PlantType::Carrot;

    fn generate(age: u32, p: &CarrotParams, season: f32, dormancy_offset: f32) -> Vec<Action> {
        generate(age, p, season, dormancy_offset)
    }

    fn max_iterations(p: &CarrotParams) -> u32 {
        p.max_iterations
    }

    fn colour(p: &CarrotParams, _iteration: u32) -> Vec3 {
        p.stem_colour
    }

    fn ui(p: &mut CarrotParams, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("carrot_params")
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
                    "Leaf depth",
                    egui::DragValue::new(&mut p.leaf_depth).range(1..=8),
                );
                changed |= widget::row(
                    ui,
                    "Rosette angle",
                    egui::Slider::new(&mut p.rosette_angle_deg, 10.0..=80.0).suffix("°"),
                );
                changed |= widget::row(
                    ui,
                    "Rachis arch",
                    egui::Slider::new(&mut p.rachis_arch_deg, 0.0..=30.0).suffix("°"),
                );
                changed |= widget::row(
                    ui,
                    "Ray angle",
                    egui::Slider::new(&mut p.ray_angle_deg, 5.0..=70.0).suffix("°"),
                );
                changed |= widget::row(
                    ui,
                    "Sub angle",
                    egui::Slider::new(&mut p.sub_angle_deg, 5.0..=60.0).suffix("°"),
                );
                changed |= widget::row(
                    ui,
                    "Leaf angle",
                    egui::Slider::new(&mut p.leaf_angle_deg, 5.0..=70.0).suffix("°"),
                );
                changed |= widget::row(
                    ui,
                    "Internode length",
                    egui::Slider::new(&mut p.i_init, 0.01..=0.15).max_decimals(3),
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
                    "Floret size",
                    egui::Slider::new(&mut p.floret_size, 0.005..=0.1).max_decimals(3),
                );
                changed |= widget::row(
                    ui,
                    "Branch radius",
                    egui::Slider::new(&mut p.branch_radius, 0.002..=0.06).max_decimals(4),
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
            *p = CarrotParams::default();
            changed = true;
        }

        changed
    }
}

#[derive(Debug, Copy, Clone)]
enum Cr {
    U(u32),
    R,
    S,
    /// Compound leaf: depth=0 is a terminal leaflet quad; depth>0 is one rachis node.
    L(u32),
    K,
    I(f32),
    Turn(f32),
    Roll(f32),
    Colour(Vec3),
    Push,
    Pop,
}

impl PartialEq for Cr {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Symbol for Cr {
    fn symbol_type(&self) -> SymbolType {
        match self {
            Cr::U(_) | Cr::R | Cr::S | Cr::L(_) | Cr::K | Cr::I(_) => SymbolType::NonTerminal,
            _ => SymbolType::Terminal,
        }
    }
}

impl Display for Cr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::U(t) => write!(f, "U({t})"),
            Self::R => write!(f, "R"),
            Self::S => write!(f, "S"),
            Self::L(d) => write!(f, "L({d})"),
            Self::K => write!(f, "K"),
            Self::I(l) => write!(f, "I({l})"),
            Self::Turn(angle) => fmt_angle(f, '+', '-', *angle),
            Self::Roll(angle) => fmt_angle(f, '/', '\\', *angle),
            Self::Colour(c) => write!(f, "C({})", c),
            Self::Push => write!(f, "["),
            Self::Pop => write!(f, "]"),
        }
    }
}

fn generate(age: u32, p: &CarrotParams, season: f32, dormancy_offset: f32) -> Vec<Action> {
    use std::f32::consts::{FRAC_PI_2, TAU};

    use Cr::*;
    const GOLDEN_ANGLE: f32 = 2.399655; // radians
    const RAY_ROLL: f32 = TAU / 5.0;
    const NUM_ROSETTE_LEAVES: u32 = 6;

    // Biennial: dormant in winter, regrows from rosette in spring.
    let age =
        (age as f32 * (1.0 - super::dormancy_factor(season, dormancy_offset, 0.65))).round() as u32;

    // Flowering: white umbels in early summer, drying to seed pods by late summer.
    let pod_colour = glam::vec3(0.50, 0.42, 0.18);
    let show_k = season < 0.68; // seed pods fall by mid-summer
    let flower_colour = if season < 0.35 {
        p.flower_colour
    } else if season < 0.58 {
        let t = (season - 0.35) / 0.23;
        let t = t * t * (3.0 - 2.0 * t);
        p.flower_colour.lerp(pod_colour, t)
    } else {
        pod_colour
    };

    let ray_angle = p.ray_angle_deg.to_radians();
    let sub_angle = p.sub_angle_deg.to_radians();
    let leaf_angle = p.leaf_angle_deg.to_radians();
    let rosette_angle = p.rosette_angle_deg.to_radians();
    let rachis_arch = p.rachis_arch_deg.to_radians();
    let i_init = p.i_init;
    let i_max = p.i_max;
    let i_growth = p.i_growth;
    let flower_delay = p.flower_delay;
    let leaf_depth = p.leaf_depth;
    let stem_colour = p.stem_colour;
    let leaf_colour = p.leaf_colour;

    let fn_i = move |s: &Cr, out: &mut Vec<Cr>| {
        if let &Cr::I(l) = s {
            out.push(I((l * i_growth).min(i_max)));
            true
        } else {
            false
        }
    };
    let rule_i = Rule::Parametric(I(0.0), &fn_i);

    // Waiting phase: U(t) → U(t+1) while t < flower_delay.
    let rule_u_wait = Rule::Parametric(U(0), &move |s: &Cr, out: &mut Vec<Cr>| {
        if let &Cr::U(t) = s
            && t < flower_delay
        {
            out.push(U(t + 1));
            true
        } else {
            false
        }
    });

    // Blooming phase: produce 5 ray branches (rng baked at rule-creation time).
    let mut rule_u_bloom_out = Vec::new();
    for i in 0..5u32 {
        rule_u_bloom_out.extend(
            [
                Push,
                Roll(i as f32 * RAY_ROLL + rng::random_range(-0.12f32, 0.12f32)),
                Turn(ray_angle + rng::random_range(-0.08f32, 0.08f32)),
                Colour(stem_colour),
                R,
                Pop,
            ]
            .repeat(5),
        );
    }
    let rule_u_bloom = Rule::Parametric(U(0), &move |s: &Cr, out: &mut Vec<Cr>| {
        if let &Cr::U(t) = s
            && t >= flower_delay
        {
            out.extend(rule_u_bloom_out.iter().copied());
            true
        } else {
            false
        }
    });

    let jr = rng::random_range(-0.1f32, 0.1f32);
    let rule_r = Rule::Normal(R, &vec![
        I(i_init),
        Push,
        Roll(GOLDEN_ANGLE + jr),
        Turn(sub_angle + jr),
        S,
        Pop,
        Push,
        Roll(-GOLDEN_ANGLE + jr),
        Turn(-(sub_angle + jr)),
        S,
        Pop,
        Push,
        Roll(GOLDEN_ANGLE * 2.0 + jr),
        Turn(sub_angle + jr),
        S,
        Pop,
        I(i_init),
    ]);

    let js = rng::random_range(-0.08f32, 0.08f32);
    let rule_s = Rule::Normal(S, &vec![
        I(i_init),
        Push,
        Roll(GOLDEN_ANGLE + js),
        Turn(sub_angle + js),
        Colour(flower_colour),
        K,
        Pop,
        Push,
        Roll(-GOLDEN_ANGLE + js),
        Turn(-(sub_angle + js)),
        Colour(flower_colour),
        K,
        Pop,
        Colour(flower_colour),
        K,
    ]);

    // L(0) = terminal leaflet — stays fixed (maps to Leaf action in translation).
    // L(d) = one rachis node: places a pair of lateral leaflets then continues to L(d-1).
    // This produces a linear pinnate structure: at most 2*leaf_depth+1 leaflets per compound leaf.
    let rule_l_terminal = Rule::Parametric(L(0), &|s: &Cr, out: &mut Vec<Cr>| {
        if let &Cr::L(d) = s
            && d == 0
        {
            out.push(L(0));
            true
        } else {
            false
        }
    });

    let rule_l_rachis = Rule::Parametric(L(0), &move |s: &Cr, out: &mut Vec<Cr>| {
        if let &Cr::L(d) = s
            && d > 0
        {
            let j = rng::random_range(-0.12f32, 0.12f32);
            out.extend([
                I(i_init),
                Push,
                Roll(FRAC_PI_2 + j),
                Turn(leaf_angle + j),
                Colour(leaf_colour),
                L(0),
                Pop,
                Push,
                Roll(-FRAC_PI_2 + j),
                Turn(leaf_angle + j),
                Colour(leaf_colour),
                L(0),
                Pop,
                Turn(-rachis_arch),
                L(d - 1),
            ]);
            true
        } else {
            false
        }
    });

    // Rosette of NUM_ROSETTE_LEAVES compound leaves radiating at golden-angle intervals,
    // plus a central umbel stem. All leaves emerge from the base (no stem height between them).
    let mut axiom = Vec::new();
    for i in 0..NUM_ROSETTE_LEAVES {
        let roll = i as f32 * GOLDEN_ANGLE;
        axiom.extend([
            Push,
            Roll(roll),
            Turn(rosette_angle),
            Colour(leaf_colour),
            L(leaf_depth),
            Pop,
        ]);
    }
    axiom.extend([Colour(stem_colour), I(i_init), U(0)]);

    let mut lsystem: LSystem<Cr> = LSystem::new(&axiom, vec![
        rule_i,
        rule_u_wait,
        rule_u_bloom,
        rule_r,
        rule_s,
        rule_l_terminal,
        rule_l_rachis,
    ]);
    lsystem.evolve(age as usize);

    let branch_radius = p.branch_radius;
    let leaf_width = p.leaf_width;
    let leaf_height = p.leaf_height;
    let floret_size = p.floret_size;

    lsystem
        .current()
        .iter()
        .map(|&s| match s {
            I(l) => Action::Branch(l, branch_radius),
            L(0) => Action::Leaf(leaf_width, leaf_height),
            L(_) => Action::Nop,
            K => {
                if show_k {
                    Action::Leaf(floret_size, floret_size)
                } else {
                    Action::Nop
                }
            }
            Turn(a) => Action::Turn(a),
            Roll(a) => Action::Roll(a),
            Colour(c) => Action::Colour(c),
            Push => Action::Push,
            Pop => Action::Pop,
            _ => Action::Nop,
        })
        .collect()
}
