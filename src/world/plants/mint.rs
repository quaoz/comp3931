//! Mint plant inspired by "The Algorithmic Beauty of Plants" fig 3.11, pg 81.
//! Decussate phyllotaxis with lateral sub-stems.

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
pub struct MintParams {
    pub stem_colour: Vec3,
    pub leaf_colour: Vec3,
    pub branch_radius: f32,
    pub leaf_width: f32,
    pub leaf_height: f32,
    pub leaf_angle_deg: f32,
    pub branch_angle_deg: f32,
    pub i_init: f32,
    pub i_max: f32,
    pub i_growth: f32,
    pub max_nodes: u32,
    pub max_iterations: u32,
}

impl Default for MintParams {
    fn default() -> Self {
        Self {
            stem_colour: vec3(0.2, 0.6, 0.15),
            leaf_colour: vec3(0.35, 0.80, 0.25),
            branch_radius: 0.022,
            leaf_width: 0.09,
            leaf_height: 0.13,
            leaf_angle_deg: 70.0,
            branch_angle_deg: 35.0,
            i_init: 0.07,
            i_max: 0.22,
            i_growth: 1.2,
            max_nodes: 6,
            max_iterations: 20,
        }
    }
}

pub struct MintPlant {
    iteration: u32,
    dirty: bool,
    cached_actions: Vec<Action>,
    pub params: MintParams,
    last_season: f32,
    dormancy_offset: f32,
}

impl Default for MintPlant {
    fn default() -> Self {
        Self::new()
    }
}

impl MintPlant {
    pub fn new() -> Self {
        let params = MintParams::default();
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

impl Plant for MintPlant {
    fn plant_type(&self) -> PlantType {
        PlantType::Mint
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

        egui::Grid::new("mint_params")
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

                ui.label("Max nodes");
                let mut n = p.max_nodes as i32;
                if ui.add(egui::Slider::new(&mut n, 1..=12)).changed() {
                    p.max_nodes = n as u32;
                    changed = true;
                }
                ui.end_row();

                ui.label("Leaf angle");
                if ui
                    .add(egui::Slider::new(&mut p.leaf_angle_deg, 10.0..=85.0).suffix("°"))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Branch angle");
                if ui
                    .add(egui::Slider::new(&mut p.branch_angle_deg, 5.0..=70.0).suffix("°"))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Internode length");
                if ui
                    .add(egui::Slider::new(&mut p.i_init, 0.01..=0.2).max_decimals(3))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Internode max");
                if ui
                    .add(egui::Slider::new(&mut p.i_max, 0.05..=0.4).max_decimals(3))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Internode growth");
                if ui
                    .add(egui::Slider::new(&mut p.i_growth, 1.0..=1.5).max_decimals(2))
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

                ui.label("Leaf width");
                changed |= ui
                    .add(egui::Slider::new(&mut p.leaf_width, 0.01..=0.25).max_decimals(3))
                    .changed();
                ui.end_row();

                ui.label("Leaf height");
                changed |= ui
                    .add(egui::Slider::new(&mut p.leaf_height, 0.01..=0.35).max_decimals(3))
                    .changed();
                ui.end_row();

                ui.label("Stem colour");
                let mut rgb = p.stem_colour.to_array();
                if ui.color_edit_button_rgb(&mut rgb).changed() {
                    p.stem_colour = Vec3::from(rgb);
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
            self.params = MintParams::default();
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
enum Ms {
    A(u32),
    B,
    I(f32),
    L,
    Turn(f32),
    Roll(f32),
    Colour(Vec3),
    Push,
    Pop,
}

impl PartialEq for Ms {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Symbol for Ms {
    fn symbol_type(&self) -> SymbolType {
        match self {
            Ms::A(_) | Ms::B | Ms::I(_) | Ms::L => SymbolType::NonTerminal,
            _ => SymbolType::Terminal,
        }
    }
}

impl Display for Ms {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::A(t) => write!(f, "A({t})"),
            Self::B => write!(f, "B"),
            Self::I(l) => write!(f, "I({l})"),
            Self::L => write!(f, "L"),
            Self::Turn(angle) => {
                if *angle >= 0.0 {
                    write!(f, "+({angle})")
                } else {
                    write!(f, "-({angle})")
                }
            }
            Self::Roll(angle) => {
                if *angle >= 0.0 {
                    write!(f, "/({angle})")
                } else {
                    write!(f, "\\({angle})")
                }
            }
            Self::Colour(c) => write!(f, "C({})", c),
            Self::Push => write!(f, "["),
            Self::Pop => write!(f, "]"),
        }
    }
}

fn generate(age: u32, p: &MintParams, season: f32, dormancy_offset: f32) -> Vec<Action> {
    // Herbaceous perennial: dies back in autumn/winter, regrows in spring.
    let age =
        (age as f32 * (1.0 - super::dormancy_factor(season, dormancy_offset, 0.65))).round() as u32;
    use std::f32::consts::FRAC_PI_2;

    use Ms::*;

    // Autumn colouration: mint leaves turn bronze as they die back.
    let autumn_leaf = glam::vec3(0.62, 0.32, 0.12);
    let leaf_colour = if season < 0.45 {
        p.leaf_colour
    } else if season < 0.82 {
        let t = (season - 0.45) / 0.37;
        let t = t * t * (3.0 - 2.0 * t);
        p.leaf_colour.lerp(autumn_leaf, t)
    } else {
        autumn_leaf
    };

    let branch_angle = p.branch_angle_deg.to_radians();
    let leaf_angle = p.leaf_angle_deg.to_radians();
    use std::f32::consts::PI;

    let rule_i = Rule::Parametric(I(0.0), &|s: &Ms, out: &mut Vec<Ms>| {
        if let &Ms::I(l) = s {
            out.push(I((l * p.i_growth).min(p.i_max)));
            true
        } else {
            false
        }
    });

    let jb = rng::random_range(-0.15f32, 0.15f32);
    let rule_b = Rule::Normal(B, &vec![
        Colour(p.stem_colour),
        I(p.i_init * 0.8),
        Push,
        Colour(leaf_colour),
        Roll(jb),
        Turn(leaf_angle),
        L,
        Pop,
        Push,
        Colour(leaf_colour),
        Roll(jb),
        Turn(-leaf_angle),
        L,
        Pop,
        Colour(p.stem_colour),
        Roll(FRAC_PI_2),
        I(p.i_init * 0.8),
        Push,
        Colour(leaf_colour),
        Roll(jb + 0.2),
        Turn(leaf_angle),
        L,
        Pop,
        Push,
        Colour(leaf_colour),
        Roll(jb + 0.2),
        Turn(-leaf_angle),
        L,
        Pop,
    ]);

    // Four A cases: (grow|term) × (no-branch|branch), conditions checked in each closure.
    let rule_a_grow_nb = Rule::Parametric(A(0), &move |s: &Ms, out: &mut Vec<Ms>| {
        if let &Ms::A(t) = s
            && t < p.max_nodes
            && t % 3 != 2
        {
            let j = rng::random_range(-0.2f32, 0.2f32);
            out.extend([
                Colour(p.stem_colour),
                I(p.i_init),
                Push,
                Colour(leaf_colour),
                Roll(j),
                Turn(leaf_angle),
                L,
                Pop,
                Push,
                Colour(leaf_colour),
                Roll(j),
                Turn(-leaf_angle),
                L,
                Pop,
                Colour(p.stem_colour),
                Roll(FRAC_PI_2),
                I(p.i_init),
                Push,
                Colour(leaf_colour),
                Roll(j + 0.3),
                Turn(leaf_angle),
                L,
                Pop,
                Push,
                Colour(leaf_colour),
                Roll(j + 0.3),
                Turn(leaf_angle),
                L,
                Pop,
                Colour(p.stem_colour),
                Roll(FRAC_PI_2),
                A(t + 1),
            ]);
            true
        } else {
            false
        }
    });

    let rule_a_grow_b = Rule::Parametric(A(0), &move |s: &Ms, out: &mut Vec<Ms>| {
        if let &Ms::A(t) = s
            && t < p.max_nodes
            && t % 3 == 2
        {
            let j = rng::random_range(-0.2f32, 0.2f32);
            out.extend([
                Colour(p.stem_colour),
                I(p.i_init),
                Push,
                Colour(leaf_colour),
                Roll(j),
                Turn(leaf_angle),
                L,
                Pop,
                Push,
                Colour(leaf_colour),
                Roll(j),
                Turn(leaf_angle),
                L,
                Pop,
                Colour(p.stem_colour),
                Roll(FRAC_PI_2),
                I(p.i_init),
                Push,
                Colour(leaf_colour),
                Roll(j + 0.3),
                Turn(leaf_angle),
                L,
                Pop,
                Push,
                Colour(leaf_colour),
                Roll(j + 0.3),
                Turn(leaf_angle),
                L,
                Pop,
                Colour(p.stem_colour),
                Roll(FRAC_PI_2),
                Push,
                Roll(PI + j * 0.5),
                Turn(branch_angle),
                B,
                Pop,
                Push,
                Roll(j * 0.5),
                Turn(branch_angle),
                B,
                Pop,
                A(t + 1),
            ]);
            true
        } else {
            false
        }
    });

    let rule_a_term_nb = Rule::Parametric(A(0), &move |s: &Ms, out: &mut Vec<Ms>| {
        if let &Ms::A(t) = s
            && t >= p.max_nodes
            && t % 3 != 2
        {
            let j = rng::random_range(-0.2f32, 0.2f32);
            out.extend([
                Colour(p.stem_colour),
                I(p.i_init),
                Push,
                Colour(leaf_colour),
                Roll(j),
                Turn(p.leaf_angle_deg.to_radians()),
                L,
                Pop,
                Push,
                Colour(leaf_colour),
                Roll(j),
                Turn(-p.leaf_angle_deg.to_radians()),
                L,
                Pop,
                Colour(p.stem_colour),
                Roll(FRAC_PI_2),
                I(p.i_init),
                Push,
                Colour(leaf_colour),
                Roll(j + 0.3),
                Turn(p.leaf_angle_deg.to_radians()),
                L,
                Pop,
                Push,
                Colour(leaf_colour),
                Roll(j + 0.3),
                Turn(-leaf_angle),
                L,
                Pop,
                Colour(p.stem_colour),
                Roll(FRAC_PI_2),
            ]);
            true
        } else {
            false
        }
    });

    let rule_a_term_b = Rule::Parametric(A(0), &move |s: &Ms, out: &mut Vec<Ms>| {
        if let &Ms::A(t) = s
            && t >= p.max_nodes
            && t % 3 == 2
        {
            let j = rng::random_range(-0.2f32, 0.2f32);
            out.extend([
                Colour(p.stem_colour),
                I(p.i_init),
                Push,
                Colour(leaf_colour),
                Roll(j),
                Turn(p.leaf_angle_deg.to_radians()),
                L,
                Pop,
                Push,
                Colour(leaf_colour),
                Roll(j),
                Turn(-p.leaf_angle_deg.to_radians()),
                L,
                Pop,
                Colour(p.stem_colour),
                Roll(FRAC_PI_2),
                I(p.i_init),
                Push,
                Colour(leaf_colour),
                Roll(j + 0.3),
                Turn(p.leaf_angle_deg.to_radians()),
                L,
                Pop,
                Push,
                Colour(leaf_colour),
                Roll(j + 0.3),
                Turn(-p.leaf_angle_deg.to_radians()),
                L,
                Pop,
                Colour(p.stem_colour),
                Roll(FRAC_PI_2),
                Push,
                Roll(PI + j * 0.5),
                Turn(p.branch_angle_deg.to_radians()),
                B,
                Pop,
                Push,
                Roll(j * 0.5),
                Turn(p.branch_angle_deg.to_radians()),
                B,
                Pop,
            ]);
            true
        } else {
            false
        }
    });

    let mut lsystem: LSystem<Ms> = LSystem::new(&[A(0)], vec![
        rule_i,
        rule_b,
        rule_a_grow_nb,
        rule_a_grow_b,
        rule_a_term_nb,
        rule_a_term_b,
    ]);
    lsystem.evolve(age as usize);

    lsystem
        .current()
        .iter()
        .map(|&s| match s {
            I(l) => Action::Branch(l, p.branch_radius),
            L => Action::Leaf(p.leaf_width, p.leaf_height),
            Turn(a) => Action::Turn(a),
            Roll(a) => Action::Roll(a),
            Colour(c) => Action::Colour(c),
            Push => Action::Push,
            Pop => Action::Pop,
            _ => Action::Nop,
        })
        .collect()
}
