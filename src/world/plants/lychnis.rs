//! Lychnis (Campion) inspired by "The Algorithmic Beauty of Plants" fig 3.14, pg 84.

use std::{
    f32::consts::{FRAC_PI_2, FRAC_PI_3},
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

pub struct LychnisPlant {
    iteration: u32,
    dirty: bool,
    cached_actions: Vec<Action>,
    pub params: LychnisParams,
    last_season: f32,
    dormancy_offset: f32,
}

impl Default for LychnisPlant {
    fn default() -> Self {
        Self::new()
    }
}

impl LychnisPlant {
    pub fn new() -> Self {
        let params = LychnisParams::default();
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

impl Plant for LychnisPlant {
    fn plant_type(&self) -> PlantType {
        PlantType::Lychnis
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

        egui::Grid::new("lychnis_params")
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
                    .add(egui::Slider::new(&mut p.branch_angle_deg, 5.0..=70.0).suffix("°"))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Internode length");
                if ui
                    .add(egui::Slider::new(&mut p.i_init, 0.005..=0.1).max_decimals(4))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Branch radius");
                if ui
                    .add(egui::Slider::new(&mut p.branch_radius, 0.005..=0.06).max_decimals(4))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Flower size");
                if ui
                    .add(egui::Slider::new(&mut p.flower_size, 0.01..=0.15).max_decimals(3))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Leaf width");
                changed |= ui
                    .add(egui::Slider::new(&mut p.leaf_width, 0.01..=0.2).max_decimals(3))
                    .changed();
                ui.end_row();

                ui.label("Leaf height");
                changed |= ui
                    .add(egui::Slider::new(&mut p.leaf_height, 0.01..=0.3).max_decimals(3))
                    .changed();
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
            self.params = LychnisParams::default();
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
            Self::Roll(angle) => {
                if *angle >= 0.0 {
                    write!(f, "/({angle})")
                } else {
                    write!(f, "\\({})", angle.abs())
                }
            }
            Self::Pitch(angle) => {
                if *angle >= 0.0 {
                    write!(f, "^({angle})")
                } else {
                    write!(f, "&({})", angle.abs())
                }
            }
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
