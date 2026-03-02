//! Tree plant with stochastic branching for natural variation.

use std::fmt::Display;

use glam::{Vec3, vec3};

use crate::{
    settings::PlantType,
    util::{
        lsystem::{LSystem, Rule, Symbol, SymbolType},
        rng,
        turtle::Action,
    },
    world::plants::{Plant, PlantEnvironment, lerp_bark_colour},
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

pub struct TreePlant {
    iteration: u32,
    dirty: bool,
    cached_actions: Vec<Action>,
    pub params: TreeParams,
    last_season: f32,
}

impl Default for TreePlant {
    fn default() -> Self {
        Self::new()
    }
}

impl TreePlant {
    pub fn new() -> Self {
        let params = TreeParams::default();
        let actions = generate(0, &params, 0.25);
        Self {
            iteration: 0,
            dirty: false,
            cached_actions: actions,
            params,
            last_season: 0.25,
        }
    }
}

impl Plant for TreePlant {
    fn plant_type(&self) -> PlantType {
        PlantType::Tree
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
        lerp_bark_colour(
            self.iteration,
            self.params.max_iterations as f32,
            self.params.young_bark,
            self.params.old_bark,
        )
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        let p = &mut self.params;

        egui::Grid::new("tree_params")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("Max iterations");
                let mut a = p.max_iterations as i32;
                if ui.add(egui::DragValue::new(&mut a).range(1..=15)).changed() {
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

                ui.label("Angle jitter");
                if ui
                    .add(egui::Slider::new(&mut p.angle_jitter, 0.0..=0.5).max_decimals(3))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Init seg length");
                if ui
                    .add(egui::Slider::new(&mut p.f_init, 0.01..=0.15).max_decimals(3))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Seg growth rate");
                if ui
                    .add(egui::Slider::new(&mut p.f_growth, 1.0..=2.0).max_decimals(2))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Max seg length");
                if ui
                    .add(egui::Slider::new(&mut p.f_max, 0.05..=0.6).max_decimals(3))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Length jitter");
                if ui
                    .add(egui::Slider::new(&mut p.f_rand, 0.0..=0.1).max_decimals(3))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Radius ratio");
                if ui
                    .add(egui::Slider::new(&mut p.branch_radius_ratio, 0.05..=0.6).max_decimals(2))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Min branch radius");
                if ui
                    .add(egui::Slider::new(&mut p.min_branch_radius, 0.001..=0.05).max_decimals(4))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Leaf width");
                changed |= ui
                    .add(egui::Slider::new(&mut p.leaf_width, 0.01..=0.3).max_decimals(3))
                    .changed();
                ui.end_row();

                ui.label("Leaf height");
                changed |= ui
                    .add(egui::Slider::new(&mut p.leaf_height, 0.01..=0.4).max_decimals(3))
                    .changed();
                ui.end_row();

                ui.label("Shed age");
                let mut sa = p.shed_age as i32;
                if ui.add(egui::Slider::new(&mut sa, 1..=15)).changed() {
                    p.shed_age = sa as u32;
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

                ui.label("Young bark");
                let mut rgb = p.young_bark.to_array();
                if ui.color_edit_button_rgb(&mut rgb).changed() {
                    p.young_bark = Vec3::from(rgb);
                    changed = true;
                }
                ui.end_row();

                ui.label("Old bark");
                let mut rgb = p.old_bark.to_array();
                if ui.color_edit_button_rgb(&mut rgb).changed() {
                    p.old_bark = Vec3::from(rgb);
                    changed = true;
                }
                ui.end_row();
            });

        if ui.button("Reset").clicked() {
            self.params = TreeParams::default();
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
        p.dirty = true;
        Box::new(p)
    }

    fn actions(&mut self, env: &PlantEnvironment) -> &[Action] {
        if (env.season - self.last_season).abs() > 0.02 {
            self.dirty = true;
        }
        if self.dirty {
            self.cached_actions = generate(self.iteration, &self.params, env.season);
            self.last_season = env.season;
            self.dirty = false;
        }
        &self.cached_actions
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
            Self::Roll(angle) => {
                if *angle >= 0.0 {
                    write!(f, "/({angle})")
                } else {
                    write!(f, "\\({angle})")
                }
            }
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

fn generate(age: u32, p: &TreeParams, season: f32) -> Vec<Action> {
    use Ts::*;

    // Deciduous leaf shedding: ramps up through autumn, peaks in winter, recovers in spring.
    // Max shed 0.85: a few leaves persist (not completely bare).
    const MAX_SHED: f64 = 0.85;
    let shed_prob: f64 = if season < 0.50 {
        0.0
    } else if season < 0.82 {
        let t = ((season - 0.50) / 0.32) as f64;
        MAX_SHED * t * t * (3.0 - 2.0 * t)
    } else if season < 0.90 {
        MAX_SHED
    } else {
        let t = ((season - 0.90) / 0.10) as f64;
        MAX_SHED * (1.0 - t * t * (3.0 - 2.0 * t))
    };

    let bark = lerp_bark_colour(age, p.max_iterations as f32, p.young_bark, p.old_bark);

    // Autumn colouration: leaves turn amber-gold before shedding.
    let autumn_leaf = glam::vec3(0.82, 0.52, 0.08);
    let leaf_colour = if season < 0.48 {
        p.leaf_colour
    } else if season < 0.78 {
        let t = (season - 0.48) / 0.30;
        let t = t * t * (3.0 - 2.0 * t);
        p.leaf_colour.lerp(autumn_leaf, t)
    } else {
        autumn_leaf
    };

    // Spring elongation: new shoots grow faster in early spring.
    let spring = if season < 0.08 {
        let t = season / 0.08;
        t * t * (3.0 - 2.0 * t)
    } else if season < 0.18 {
        let t = (season - 0.08) / 0.10;
        1.0 - t * t * (3.0 - 2.0 * t)
    } else {
        0.0
    };

    let shedding = age >= p.shed_age;
    let branch_angle = p.branch_angle_deg.to_radians();
    let angle_jitter = p.angle_jitter;
    let f_init = p.f_init;
    let f_max = p.f_max;
    let f_rand = p.f_rand;
    let f_growth = p.f_growth * (1.0 + 0.20 * spring);
    let d_init = f_init * p.branch_radius_ratio;
    let d_max = f_max * p.branch_radius_ratio;
    let min_radius = p.min_branch_radius;

    let green = SetColour(leaf_colour);
    let brown = SetColour(bark);

    let rule_f = Rule::Parametric(F(0.0, 0.0), &|s: &Ts, out: &mut Vec<Ts>| {
        if let &Ts::F(l, d) = s {
            let r = rng::random_range(-f_rand, f_rand);
            out.push(F((l * f_growth + r).min(f_max), (d * f_growth).min(d_max)));
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

    let jt = Roll(rng::random_range(-angle_jitter, angle_jitter));
    let rule_t = Rule::Normal(T, &vec![
        F(f_init, d_init),
        F(f_init, d_init),
        F(f_init, d_init),
        jt,
        Push,
        Rl,
        Tl,
        F(f_init, d_init),
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
        F(f_init, d_init),
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
        F(f_init, d_init),
        X,
        Push,
        Tr,
        G,
        Pop,
    ]);

    let jx = Roll(rng::random_range(-angle_jitter, angle_jitter));
    let rule_x = Rule::Normal(X, &vec![
        F(f_init, d_init),
        jx,
        Push,
        Rl,
        Tl,
        F(f_init, d_init),
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
        F(f_init, d_init),
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
        F(f_init, d_init),
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
        F(l, d) => Action::Branch(l, d.max(min_radius)),
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
