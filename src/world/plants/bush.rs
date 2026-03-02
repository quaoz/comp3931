//! Bush plant — compact three-branch template with seasonal leaf scaling.

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

pub struct BushPlant {
    iteration: u32,
    dirty: bool,
    cached_actions: Vec<Action>,
    pub params: BushParams,
    last_season: f32,
}

impl Default for BushPlant {
    fn default() -> Self {
        Self::new()
    }
}

impl BushPlant {
    pub fn new() -> Self {
        let params = BushParams::default();
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

impl Plant for BushPlant {
    fn plant_type(&self) -> PlantType {
        PlantType::Bush
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
        let p = &mut self.params;

        egui::Grid::new("bush_params")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("Max iterations");
                if ui
                    .add(egui::DragValue::new(&mut p.max_iterations).range(1..=12))
                    .changed()
                {
                    self.dirty = true;
                }
                ui.end_row();

                ui.label("Branch angle");
                if ui
                    .add(egui::Slider::new(&mut p.branch_angle_deg, 5.0..=60.0).suffix("°"))
                    .changed()
                {
                    self.dirty = true;
                }
                ui.end_row();

                ui.label("Branch length");
                if ui
                    .add(egui::Slider::new(&mut p.branch_len, 0.01..=0.2).max_decimals(3))
                    .changed()
                {
                    self.dirty = true;
                }
                ui.end_row();

                ui.label("Branch radius");
                if ui
                    .add(egui::Slider::new(&mut p.branch_radius, 0.002..=0.05).max_decimals(3))
                    .changed()
                {
                    self.dirty = true;
                }
                ui.end_row();

                ui.label("Trunk jitter");
                if ui
                    .add(egui::Slider::new(&mut p.jitter_trunk, 0.0..=0.5).max_decimals(3))
                    .changed()
                {
                    self.dirty = true;
                }
                ui.end_row();

                ui.label("Branch jitter");
                if ui
                    .add(egui::Slider::new(&mut p.jitter_branch, 0.0..=0.4).max_decimals(3))
                    .changed()
                {
                    self.dirty = true;
                }
                ui.end_row();

                ui.label("Leaf width");
                self.dirty |= ui
                    .add(egui::Slider::new(&mut p.leaf_width, 0.01..=0.2).max_decimals(3))
                    .changed();
                ui.end_row();

                ui.label("Leaf height");
                self.dirty |= ui
                    .add(egui::Slider::new(&mut p.leaf_height, 0.01..=0.3).max_decimals(3))
                    .changed();
                ui.end_row();

                ui.label("Leaf colour");
                let mut rgb = p.leaf_colour.to_array();
                if ui.color_edit_button_rgb(&mut rgb).changed() {
                    p.leaf_colour = Vec3::from(rgb);
                    self.dirty = true;
                }
                ui.end_row();

                ui.label("Young bark");
                let mut rgb = p.young_bark.to_array();
                if ui.color_edit_button_rgb(&mut rgb).changed() {
                    p.young_bark = Vec3::from(rgb);
                    self.dirty = true;
                }
                ui.end_row();

                ui.label("Old bark");
                let mut rgb = p.old_bark.to_array();
                if ui.color_edit_button_rgb(&mut rgb).changed() {
                    p.old_bark = Vec3::from(rgb);
                    self.dirty = true;
                }
                ui.end_row();
            });

        if ui.button("Reset").clicked() {
            self.params = BushParams::default();
            self.dirty = true;
        }

        self.dirty
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
            Self::Roll(angle) => {
                if *angle >= 0.0 {
                    write!(f, "/({angle})")
                } else {
                    write!(f, "\\({angle})")
                }
            }
            Self::Rl => write!(f, "Rl"),
            Self::Tl => write!(f, "Tl"),
            Self::Tr => write!(f, "Tr"),
            Self::Push => write!(f, "["),
            Self::Pop => write!(f, "]"),
            Self::SetColour(c) => write!(f, "C({})", c),
        }
    }
}

fn generate(age: u32, p: &BushParams, season: f32) -> Vec<Action> {
    use Bs::*;

    // Deciduous shrub: retains leaves spring/summer, drops through autumn.
    // Min 0.30: some foliage persists through winter.
    const MIN: f32 = 0.30;
    let leaf_scale = if season < 0.50 {
        1.0
    } else if season < 0.82 {
        let t = (season - 0.50) / 0.32;
        1.0 - (1.0 - MIN) * t * t * (3.0 - 2.0 * t)
    } else if season < 0.90 {
        MIN
    } else {
        let t = (season - 0.90) / 0.10;
        MIN + (1.0 - MIN) * t * t * (3.0 - 2.0 * t)
    };

    // Autumn colouration: leaves turn yellow-gold before dropping.
    let autumn_leaf = glam::vec3(0.88, 0.72, 0.08);
    let leaf_colour = if season < 0.48 {
        p.leaf_colour
    } else if season < 0.78 {
        let t = (season - 0.48) / 0.30;
        let t = t * t * (3.0 - 2.0 * t);
        p.leaf_colour.lerp(autumn_leaf, t)
    } else {
        autumn_leaf
    };

    // Spring elongation: slightly longer branches as growth resumes.
    let spring = if season < 0.08 {
        let t = season / 0.08;
        t * t * (3.0 - 2.0 * t)
    } else if season < 0.18 {
        let t = (season - 0.08) / 0.10;
        1.0 - t * t * (3.0 - 2.0 * t)
    } else {
        0.0
    };

    let branch_angle = p.branch_angle_deg.to_radians();
    let jt = p.jitter_trunk;
    let jb = p.jitter_branch;
    let bark = lerp_bark_colour(age, p.max_iterations as f32, p.young_bark, p.old_bark);
    let branch_len = p.branch_len * (1.0 + 0.15 * spring);

    let green = SetColour(leaf_colour);
    let brown = SetColour(bark);

    let leaf = move |orients: &[Bs]| -> Vec<Bs> {
        let mut v = vec![green, Push];
        v.extend_from_slice(orients);
        v.push(L);
        v.extend([Pop, brown]);
        v
    };

    let jt_val = Roll(rng::random_range(-jt, jt));
    let rule_t_out = vec![F, jt_val, X];
    let rule_t = Rule::Normal(T, &rule_t_out);

    let jx_val = Roll(rng::random_range(-jb, jb));
    let leaf_empty = leaf(&[]);
    let mut rule_x_out = vec![F, jx_val];
    rule_x_out.extend([Push, Rl, Tl, F, B, Push, Tr, Tr]);
    rule_x_out.extend(leaf_empty.iter().copied());
    rule_x_out.extend([Pop, Pop, Push, Rl, Rl, Rl, Rl, Tl, F, B, Push, Tl, Tl]);
    rule_x_out.extend(leaf_empty.iter().copied());
    rule_x_out.extend([
        Pop, Pop, Rl, Rl, Rl, Rl, Rl, Rl, Rl, Rl, Rl, Tl, F, B, Push, Tr,
    ]);
    rule_x_out.extend(leaf_empty.iter().copied());
    rule_x_out.push(Pop);
    let rule_x = Rule::Normal(X, &rule_x_out);

    let jb_val = Roll(rng::random_range(-jb, jb));
    let mut rule_b_out = vec![F, F, jb_val];
    rule_b_out.extend([Push, Rl, Tl, F, B, Push, Tr]);
    rule_b_out.extend(leaf_empty.iter().copied());
    rule_b_out.push(Tr);
    rule_b_out.extend(leaf_empty.iter().copied());
    rule_b_out.extend([Pop, Pop, Push, Rl, Rl, Rl, Rl, F, Push, Tl]);
    rule_b_out.extend(leaf_empty.iter().copied());
    rule_b_out.push(Tl);
    rule_b_out.extend(leaf_empty.iter().copied());
    rule_b_out.extend([Pop, B, Push, Tl]);
    rule_b_out.extend(leaf_empty.iter().copied());
    rule_b_out.push(Tl);
    rule_b_out.extend(leaf_empty.iter().copied());
    rule_b_out.extend([
        Pop, Pop, Rl, Rl, Rl, Rl, Rl, Rl, Rl, Rl, Rl, Tl, F, Push, Tl,
    ]);
    rule_b_out.extend(leaf_empty.iter().copied());
    rule_b_out.push(Tl);
    rule_b_out.extend(leaf_empty.iter().copied());
    rule_b_out.extend([Pop, B]);
    let rule_b = Rule::Normal(B, &rule_b_out);

    let mut lsystem = LSystem::new(&[T], vec![rule_t, rule_x, rule_b]);
    lsystem.evolve(age as usize);

    let branch_radius = p.branch_radius;
    let leaf_width = p.leaf_width;
    let leaf_height = p.leaf_height;

    let mut actions = vec![Action::Colour(bark)];
    actions.extend(lsystem.current().iter().map(|&s| match s {
        F => Action::Branch(branch_len, branch_radius),
        L => {
            if leaf_scale < 0.01 {
                Action::Nop
            } else {
                Action::Leaf(leaf_width * leaf_scale, leaf_height * leaf_scale)
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
