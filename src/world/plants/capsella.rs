/// Capsella (Shepherd's Purse) inspired by "The Algorithmic Beauty of Plants" fig 3.5, pg 74.
/// Parametric L-system modelling a herbaceous plant with alternate branches
/// developing from an apex, each terminating in a silique (seed pod).
use glam::{Vec3, vec3};

use crate::{
    settings::PlantType,
    util::{
        lsystem::{LSystem, Rule, Symbol, SymbolType},
        turtle::Action,
    },
    world::plants::Plant,
};

const GOLDEN_ANGLE: f32 = 2.399655;
const STEM_COLOUR: Vec3 = vec3(0.3, 0.65, 0.2);
const FLOWER_COLOUR: Vec3 = vec3(0.95, 0.95, 0.95);

#[derive(Debug, Copy, Clone)]
enum Cs {
    A(u32), // apex with developmental stage
    I(f32), // internode (carries current length)
    L,      // leaf
    K,      // silique / flower
    Turn(f32),
    Roll(f32),
    Colour(Vec3),
    Push,
    Pop,
}

impl PartialEq for Cs {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Symbol for Cs {
    fn symbol_type(&self) -> SymbolType {
        match self {
            Cs::A(_) | Cs::I(_) | Cs::L | Cs::K => SymbolType::NonTerminal,
            _ => SymbolType::Terminal,
        }
    }
}

impl From<Cs> for Action {
    fn from(s: Cs) -> Self {
        match s {
            Cs::I(l) => Action::Branch(l, 0.025),
            Cs::L => Action::Leaf(0.08, 0.12),
            Cs::K => Action::Leaf(0.04, 0.04),
            Cs::Turn(a) => Action::Turn(a),
            Cs::Roll(a) => Action::Roll(a),
            Cs::Colour(c) => Action::Colour(c),
            Cs::Push => Action::Push,
            Cs::Pop => Action::Pop,
            _ => Action::Nop,
        }
    }
}

pub struct CapsellaPant {
    age: u32,
    dirty: bool,
    cached_actions: Vec<Action>,
}

impl CapsellaPant {
    pub fn new(age: u32) -> Self {
        let actions = generate(age);
        Self {
            age,
            dirty: false,
            cached_actions: actions,
        }
    }
}

impl Plant for CapsellaPant {
    fn plant_type(&self) -> PlantType {
        PlantType::Capsella
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
    use Cs::*;
    let a = 30.0f32.to_radians();
    let max_branches = 3u32;
    let i_init = 0.05f32;
    let i_max = 0.16f32;

    let axiom = [A(0)];

    let rule_i = Rule::Parametric(I(0.0), &move |s, out| {
        let &Cs::I(l) = s else { unreachable!() };
        out.extend([I((l * 1.2).min(i_max))]);
        true
    });

    // A(t): at each stage produce an internode, alternate leaf pair, roll, and recurse.
    // After max_branches stages, produce a terminal inflorescence.
    let rule_a = Rule::Parametric(A(0), &move |s, out| {
        let &Cs::A(t) = s else { unreachable!() };
        if t < max_branches {
            out.extend([
                I(i_init),
                Push,
                Roll(GOLDEN_ANGLE),
                Turn(a),
                Colour(STEM_COLOUR),
                L,
                Pop,
                Push,
                Roll(-GOLDEN_ANGLE),
                Turn(-a),
                Colour(STEM_COLOUR),
                L,
                Pop,
                A(t + 1),
            ])
        } else {
            // Terminal inflorescence: a cluster of siliques/flowers
            out.extend([
                Push,
                Turn(a),
                Colour(FLOWER_COLOUR),
                K,
                Pop,
                Push,
                Roll(GOLDEN_ANGLE * 2.0),
                Turn(a),
                Colour(FLOWER_COLOUR),
                K,
                Pop,
                Push,
                Roll(GOLDEN_ANGLE * 4.0),
                Turn(a),
                Colour(FLOWER_COLOUR),
                K,
                Pop,
            ])
        }

        true
    });

    // Parametric rule must come before the A rule so I(l) is rewritten before A consumes context
    let mut lsystem: LSystem<Cs> = LSystem::new(&axiom, vec![rule_i, rule_a]);
    lsystem.evolve(age as usize);
    lsystem.current().iter().map(|s| Action::from(*s)).collect()
}
