/// Mint plant inspired by "The Algorithmic Beauty of Plants" fig 3.11, pg 81.
/// Models decussate phyllotaxis: opposite leaf pairs at each node, rotated 90° between nodes.
use glam::{Vec3, vec3};

use crate::{
    settings::PlantType,
    util::{
        lsystem::{LSystem, Rule, Symbol, SymbolType},
        turtle::Action,
    },
    world::plants::Plant,
};

const STEM_COLOUR: Vec3 = vec3(0.2, 0.6, 0.15);
const LEAF_COLOUR: Vec3 = vec3(0.35, 0.80, 0.25);

#[derive(Debug, Copy, Clone)]
enum Ms {
    A,      // growing apex
    I(f32), // internode (carries current length)
    L,      // leaf
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
            Ms::A | Ms::I(_) | Ms::L => SymbolType::NonTerminal,
            _ => SymbolType::Terminal,
        }
    }
}

impl From<Ms> for Action {
    fn from(s: Ms) -> Self {
        match s {
            Ms::I(l) => Action::Branch(l, 0.022),
            Ms::L => Action::Leaf(0.09, 0.13),
            Ms::Turn(a) => Action::Turn(a),
            Ms::Roll(a) => Action::Roll(a),
            Ms::Colour(c) => Action::Colour(c),
            Ms::Push => Action::Push,
            Ms::Pop => Action::Pop,
            _ => Action::Nop,
        }
    }
}

pub struct MintPlant {
    age: u32,
    dirty: bool,
    cached_actions: Vec<Action>,
}

impl MintPlant {
    pub fn new(age: u32) -> Self {
        let actions = generate(age);
        Self {
            age,
            dirty: false,
            cached_actions: actions,
        }
    }
}

impl Plant for MintPlant {
    fn plant_type(&self) -> PlantType {
        PlantType::Mint
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

fn generate(age: u32) -> Vec<Action> {
    use Ms::*;
    let leaf_angle = 70.0f32.to_radians();
    let half_turn = std::f32::consts::FRAC_PI_2; // 90° roll between node pairs
    let i_init = 0.07f32;
    let i_max = 0.22f32;

    let rule_i = Rule::Parametric(I(0.0), &move |s, out| {
        let &Ms::I(l) = s else { unreachable!() };
        out.extend([I((l * 1.2).min(i_max))]);
        true
    });

    // Decussate phyllotaxis: each node has two opposite leaves, next node rotated 90°.
    // A → I [+L][-L] Roll(90°) I [+L][-L] Roll(90°) A
    let rule_a = Rule::Normal(A, &[
        I(i_init),
        Push,
        Colour(LEAF_COLOUR),
        Turn(leaf_angle),
        L,
        Pop,
        Push,
        Colour(LEAF_COLOUR),
        Turn(-leaf_angle),
        L,
        Pop,
        Colour(STEM_COLOUR),
        Roll(half_turn),
        I(i_init),
        Push,
        Colour(LEAF_COLOUR),
        Turn(leaf_angle),
        L,
        Pop,
        Push,
        Colour(LEAF_COLOUR),
        Turn(-leaf_angle),
        L,
        Pop,
        Colour(STEM_COLOUR),
        Roll(half_turn),
        A,
    ]);

    let mut lsystem: LSystem<Ms> = LSystem::new(&[A], vec![rule_i, rule_a]);
    lsystem.evolve(age as usize);
    lsystem.current().iter().map(|s| Action::from(*s)).collect()
}
