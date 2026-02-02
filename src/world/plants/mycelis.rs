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
    world::plants::Plant,
};

const STEM_COLOUR: Vec3 = vec3(0.4, 0.6, 0.2);
const FLOWER_COLOUR: Vec3 = vec3(0.9, 0.85, 0.1);

pub struct MycelisPlant {
    age: u32,
    dirty: bool,
    cached_actions: Vec<Action>,
}

impl MycelisPlant {
    pub fn new(age: u32) -> Self {
        let actions = generate(age);
        Self {
            age,
            dirty: false,
            cached_actions: actions,
        }
    }
}

impl Plant for MycelisPlant {
    fn plant_type(&self) -> PlantType {
        PlantType::Mycelis
    }

    fn age(&self) -> u32 {
        self.age
    }

    fn max_age(&self) -> u32 {
        8
    }

    fn set_age(&mut self, age: u32) {
        if self.age != age {
            self.age = age;
            self.dirty = true;
        }
    }

    fn colour(&self) -> Vec3 {
        STEM_COLOUR
    }

    fn clone_boxed(&self) -> Box<dyn Plant> {
        Box::new(Self::new(self.age))
    }

    fn actions(&mut self) -> &[Action] {
        if self.dirty {
            self.cached_actions = generate(self.age);
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

fn generate(age: u32) -> Vec<Action> {
    let branch_angle = 28.0f32.to_radians();
    let i_init = 0.07f32;

    use Mys::*;

    const GREEN: Vec3 = vec3(0.07, 0.1, 0.07);
    const PURPLE: Vec3 = vec3(0.15, 0.14, 0.19);
    const FLOWER_MAX: u8 = 7;

    let rule_1 = Rule::ContextSensitive(A(0), Some(S), None, &[
        T,
        V,
        Colour(FLOWER_COLOUR),
        K(FLOWER_MAX),
        Colour(GREEN),
    ]);
    let rule_2 = Rule::ContextSensitive(A(0), Some(V), None, &[
        T,
        V,
        Colour(FLOWER_COLOUR),
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
                Turn(FRAC_PI_8 + branch_angle),
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
                    6 | 5 => (1.0, FLOWER_COLOUR),
                    4 | 3 => (0.8, FLOWER_COLOUR),
                    2 | 1 => (0.8, vec3(0.8, 0.3, 0.1)),
                    0 => (0.5, vec3(0.4, 0.2, 0.05)),
                    _ => (0.5, FLOWER_COLOUR),
                };

                vec![
                    Action::Colour(colour),
                    Action::Leaf(scale * 0.03, scale * 0.03),
                    Action::Colour(GREEN),
                ]
            } else if Leaf == s {
                vec![
                    Action::Colour(STEM_COLOUR),
                    Action::Leaf(0.02, 0.08),
                    Action::Colour(STEM_COLOUR),
                ]
            } else {
                vec![match s {
                    G | F => Action::Branch(i_init, 0.02),
                    M => Action::Colour(STEM_COLOUR),
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
