/// Lychnis (Campion) inspired by "The Algorithmic Beauty of Plants" fig 3.14, pg 84.
/// Models a dichasial cyme: each apex produces two lateral branches and a terminal flower.
use glam::{Vec3, vec3};

use crate::{
    settings::PlantType,
    util::{
        lsystem::{LSystem, Rule, Symbol, SymbolType},
        turtle::Action,
    },
    world::plants::Plant,
};

const STEM_COLOUR: Vec3 = vec3(0.35, 0.55, 0.25);
const FLOWER_COLOUR: Vec3 = vec3(0.9, 0.2, 0.4);
const LEAF_COLOUR: Vec3 = vec3(0.3, 0.65, 0.2);

#[derive(Debug, Copy, Clone)]
enum Ls {
    A,      // vegetative apex (produces two branches + flower)
    I(f32), // internode (carries current length)
    K,      // flower
    L,      // leaf
    Turn(f32),
    Roll(f32),
    Colour(Vec3),
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
            Ls::A | Ls::I(_) | Ls::K | Ls::L => SymbolType::NonTerminal,
            _ => SymbolType::Terminal,
        }
    }
}

impl From<Ls> for Action {
    fn from(s: Ls) -> Self {
        match s {
            Ls::I(l) => Action::Branch(l, 0.025),
            Ls::K => Action::Leaf(0.05, 0.05),
            Ls::L => Action::Leaf(0.08, 0.12),
            Ls::Turn(a) => Action::Turn(a),
            Ls::Roll(a) => Action::Roll(a),
            Ls::Colour(c) => Action::Colour(c),
            Ls::Push => Action::Push,
            Ls::Pop => Action::Pop,
            _ => Action::Nop,
        }
    }
}

pub struct LychnisPant {
    age: u32,
    dirty: bool,
    cached_actions: Vec<Action>,
}

impl LychnisPant {
    pub fn new(age: u32) -> Self {
        let actions = generate(age);
        Self {
            age,
            dirty: false,
            cached_actions: actions,
        }
    }
}

impl Plant for LychnisPant {
    fn plant_type(&self) -> PlantType {
        PlantType::Lychnis
    }

    fn age(&self) -> u32 {
        self.age
    }

    fn max_age(&self) -> u32 {
        6
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

fn generate(age: u32) -> Vec<Action> {
    use Ls::*;
    let branch_angle = 35.0f32.to_radians();
    let half_roll = std::f32::consts::FRAC_PI_2;
    let i_init = 0.08f32;
    let i_max = 0.24f32;

    let rule_i = Rule::Parametric(I(0.0), &move |s, out| {
        let &Ls::I(l) = s else { unreachable!() };
        out.extend([I((l * 1.2).min(i_max))]);
        true
    });

    // Dichasial cyme: A → I [/+A][/-A] K
    // Each apex produces: internode, two lateral sub-apices rotated 90° apart, terminal flower
    let rule_a = Rule::Normal(A, &[
        Colour(STEM_COLOUR),
        I(i_init),
        Push,
        Roll(half_roll),
        Colour(LEAF_COLOUR),
        L,
        Pop, // subtending leaf pair
        Push,
        Roll(half_roll),
        Turn(branch_angle),
        Colour(STEM_COLOUR),
        A,
        Pop,
        Push,
        Roll(-half_roll),
        Turn(-branch_angle),
        Colour(STEM_COLOUR),
        A,
        Pop,
        Colour(FLOWER_COLOUR),
        K, // terminal flower
    ]);

    let mut lsystem: LSystem<Ls> = LSystem::new(&[A], vec![rule_i, rule_a]);
    lsystem.evolve(age as usize);
    lsystem.current().iter().map(|s| Action::from(*s)).collect()
}
