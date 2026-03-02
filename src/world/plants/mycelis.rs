//! Mycelis (Wall Lettuce) inspired by "The Algorithmic Beauty of Plants" fig 3.18-3.19, pg 90-91.
//! Context-sensitive rules model acropetal flowering propagation.

use std::{
    f32::consts::{FRAC_PI_6, FRAC_PI_8, PI},
    fmt::Display,
};

use glam::{Vec3, vec3};

use crate::{
    settings::PlantType,
    util::{
        lsystem::{ContextAction, LSystem, Rule, Symbol, SymbolType},
        rng,
        turtle::Action,
    },
    world::plants::{Plant, PlantEnvironment},
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

pub struct MycelisPlant {
    iteration: u32,
    dirty: bool,
    cached_actions: Vec<Action>,
    pub params: MycelisParams,
    last_season: f32,
    dormancy_offset: f32,
}

impl Default for MycelisPlant {
    fn default() -> Self {
        Self::new()
    }
}

impl MycelisPlant {
    pub fn new() -> Self {
        let params = MycelisParams::default();
        let dormancy_offset = rng::random_range(-0.05_f32, 0.05);
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

impl Plant for MycelisPlant {
    fn plant_type(&self) -> PlantType {
        PlantType::Mycelis
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

        egui::Grid::new("mycelis_params")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("Max iterations");
                let mut a = p.max_iterations as i32;
                if ui
                    .add(egui::DragValue::new(&mut a).range(1..=100))
                    .changed()
                {
                    p.max_iterations = a as u32;
                    changed = true;
                }
                ui.end_row();

                ui.label("Branch angle");
                if ui
                    .add(egui::Slider::new(&mut p.branch_angle_deg, 5.0..=60.0).suffix("°"))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Internode length");
                if ui
                    .add(egui::Slider::new(&mut p.i_init, 0.02..=0.3).max_decimals(3))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Branch radius");
                if ui
                    .add(egui::Slider::new(&mut p.branch_radius, 0.001..=0.03).max_decimals(4))
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
            });

        if ui.button("Reset").clicked() {
            self.params = MycelisParams::default();
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
            Self::Turn(angle) => {
                if *angle >= 0.0 {
                    write!(f, "+({angle})")
                } else {
                    write!(f, "-({})", angle.abs())
                }
            }
            Self::Roll(angle) => {
                if *angle >= 0.0 {
                    write!(f, "/({angle})")
                } else {
                    write!(f, "\\({})", angle.abs())
                }
            }
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
