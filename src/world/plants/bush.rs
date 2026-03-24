//! Bush plant — compact three-branch template with seasonal leaf scaling.

use std::fmt::Display;

use glam::{Vec3, vec3};

use crate::{
    settings::PlantType,
    util::{
        lsystem::{LSystem, Rule, Symbol, SymbolType, fmt_angle},
        rng::random_range,
        turtle::Action,
        widget,
    },
    world::plants::{Species, lerp_bark_colour},
};

#[derive(Clone)]
pub struct BushParams {
    pub max_iterations: u32,
    pub leaf_colour: Vec3,
    pub young_bark: Vec3,
    pub old_bark: Vec3,
    pub branch_angle_deg: f32,
    pub branch_len: f32,
    pub branch_radius: f32,
    pub leaf_width: f32,
    pub leaf_height: f32,
    pub jitter_trunk: f32,
    pub jitter_branch: f32,
}

impl Default for BushParams {
    fn default() -> Self {
        Self {
            max_iterations: 8,
            leaf_colour: vec3(0.25, 0.62, 0.15),
            young_bark: vec3(0.50, 0.32, 0.14),
            old_bark: vec3(0.26, 0.18, 0.10),
            branch_angle_deg: 35.7,
            branch_len: 0.04,
            branch_radius: 0.015,
            leaf_width: 0.06,
            leaf_height: 0.09,
            jitter_trunk: 0.15,
            jitter_branch: 0.10,
        }
    }
}

pub struct Bush;

impl Species for Bush {
    type Params = BushParams;

    const TYPE: PlantType = PlantType::Bush;

    fn generate(age: u32, p: &BushParams, season: f32, dormancy_offset: f32) -> Vec<Action> {
        generate(age, p, season, dormancy_offset)
    }

    fn max_iterations(p: &BushParams) -> u32 {
        p.max_iterations
    }

    fn colour(p: &BushParams, iteration: u32) -> Vec3 {
        lerp_bark_colour(iteration, p.max_iterations as f32, p.young_bark, p.old_bark)
    }

    fn ui(p: &mut BushParams, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("bush_params")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                changed |= widget::row(
                    ui,
                    "Max iterations",
                    egui::DragValue::new(&mut p.max_iterations).range(1..=12),
                );
                changed |= widget::row(
                    ui,
                    "Branch angle",
                    egui::Slider::new(&mut p.branch_angle_deg, 5.0..=60.0).suffix("°"),
                );
                changed |= widget::row(
                    ui,
                    "Branch length",
                    egui::Slider::new(&mut p.branch_len, 0.01..=0.2).max_decimals(3),
                );
                changed |= widget::row(
                    ui,
                    "Branch radius",
                    egui::Slider::new(&mut p.branch_radius, 0.002..=0.05).max_decimals(3),
                );
                changed |= widget::row(
                    ui,
                    "Trunk jitter",
                    egui::Slider::new(&mut p.jitter_trunk, 0.0..=0.5).max_decimals(3),
                );
                changed |= widget::row(
                    ui,
                    "Branch jitter",
                    egui::Slider::new(&mut p.jitter_branch, 0.0..=0.4).max_decimals(3),
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
                changed |= widget::colour_row(ui, "Leaf colour", &mut p.leaf_colour);
                changed |= widget::colour_row(ui, "Young bark", &mut p.young_bark);
                changed |= widget::colour_row(ui, "Old bark", &mut p.old_bark);
            });

        if ui.button("Reset").clicked() {
            *p = BushParams::default();
            changed = true;
        }

        changed
    }
}

#[derive(Debug, Copy, Clone)]
enum Bs {
    T,
    X,
    B,
    F,
    L,
    Roll(f32),
    Rl,
    Tl,
    Tr,
    Push,
    Pop,
    SetColour(Vec3),
}

impl PartialEq for Bs {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Symbol for Bs {
    fn symbol_type(&self) -> SymbolType {
        match self {
            Bs::T | Bs::X | Bs::B | Bs::F => SymbolType::NonTerminal,
            _ => SymbolType::Terminal,
        }
    }
}

impl Display for Bs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::T => write!(f, "T"),
            Self::X => write!(f, "X"),
            Self::B => write!(f, "B"),
            Self::F => write!(f, "F"),
            Self::L => write!(f, "L"),
            Self::Roll(angle) => fmt_angle(f, '/', '\\', *angle),
            Self::Rl => write!(f, "Rl"),
            Self::Tl => write!(f, "Tl"),
            Self::Tr => write!(f, "Tr"),
            Self::Push => write!(f, "["),
            Self::Pop => write!(f, "]"),
            Self::SetColour(c) => write!(f, "C({})", c),
        }
    }
}

fn generate(age: u32, p: &BushParams, season: f32, dormancy_offset: f32) -> Vec<Action> {
    use Bs::*;

    // Leaves turn yellow then drop through autumn
    let leaf_scale = 1.0 - super::dormancy_factor(season, dormancy_offset, 0.70);
    let leaf_colour =
        super::autumn_colour(season, p.leaf_colour, vec3(0.88, 0.72, 0.08), 0.48, 0.78);

    let spring = super::spring_surge(season);

    let branch_angle = p.branch_angle_deg.to_radians();
    let bark = lerp_bark_colour(age, p.max_iterations as f32, p.young_bark, p.old_bark);
    let branch_len = p.branch_len * (1.0 + 0.15 * spring);

    let green = SetColour(leaf_colour);
    let brown = SetColour(bark);

    let _leaf = move |orients: &[Bs]| -> Vec<Bs> {
        let mut v = vec![green, Push];
        v.extend_from_slice(orients);
        v.push(L);
        v.extend([Pop, brown]);
        v
    };

    let rule_t = Rule::Normal(T, &[
        F,
        Roll(random_range(-p.jitter_trunk, p.jitter_trunk)),
        X,
    ]);

    let leaf_empty = [green, Push, L, Pop, brown];

    let mut rule_x_out = vec![
        F,
        Roll(random_range(-p.jitter_branch, p.jitter_branch)),
        Push,
        Rl,
        Tl,
        F,
        B,
        Push,
        Tr,
        Tr,
    ];
    rule_x_out.extend(leaf_empty);
    rule_x_out.extend([Pop, Pop, Push, Rl, Rl, Rl, Rl, Tl, F, B, Push, Tl, Tl]);
    rule_x_out.extend(leaf_empty);
    rule_x_out.extend([
        Pop, Pop, Rl, Rl, Rl, Rl, Rl, Rl, Rl, Rl, Rl, Tl, F, B, Push, Tr,
    ]);
    rule_x_out.extend(leaf_empty);
    rule_x_out.push(Pop);
    let rule_x = Rule::Normal(X, &rule_x_out);

    let mut rule_b_out = vec![F, F, Roll(random_range(-p.jitter_branch, p.jitter_branch))];
    rule_b_out.extend([Push, Rl, Tl, F, B, Push, Tr]);
    rule_b_out.extend(leaf_empty);
    rule_b_out.push(Tr);
    rule_b_out.extend(leaf_empty);
    rule_b_out.extend([Pop, Pop, Push, Rl, Rl, Rl, Rl, F, Push, Tl]);
    rule_b_out.extend(leaf_empty);
    rule_b_out.push(Tl);
    rule_b_out.extend(leaf_empty);
    rule_b_out.extend([Pop, B, Push, Tl]);
    rule_b_out.extend(leaf_empty);
    rule_b_out.push(Tl);
    rule_b_out.extend(leaf_empty);
    rule_b_out.extend([
        Pop, Pop, Rl, Rl, Rl, Rl, Rl, Rl, Rl, Rl, Rl, Tl, F, Push, Tl,
    ]);
    rule_b_out.extend(leaf_empty);
    rule_b_out.push(Tl);
    rule_b_out.extend(leaf_empty);
    rule_b_out.extend([Pop, B]);
    let rule_b = Rule::Normal(B, &rule_b_out);

    let mut lsystem = LSystem::new(&[T], vec![rule_t, rule_x, rule_b]);
    lsystem.evolve(age as usize);

    let mut actions = vec![Action::Colour(bark)];
    actions.extend(lsystem.current().iter().map(|&s| match s {
        F => Action::Branch(branch_len, p.branch_radius),
        L => {
            if leaf_scale < 0.01 {
                Action::Nop
            } else {
                Action::Leaf(p.leaf_width * leaf_scale, p.leaf_height * leaf_scale)
            }
        }
        Roll(r) => Action::Roll(r),
        Rl => Action::Roll(-branch_angle),
        Tl => Action::Turn(-branch_angle),
        Tr => Action::Turn(branch_angle),
        Push => Action::Push,
        Pop => Action::Pop,
        SetColour(c) => Action::Colour(c),
        _ => Action::Nop,
    }));
    actions
}
