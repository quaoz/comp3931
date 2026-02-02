/// Fern plant inspired by "The Algorithmic Beauty of Plants" fig 8.2, pg 178.
/// Models a compound pinnate frond using bilateral branching with golden-angle phyllotaxis.
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
const DEFAULT_COLOUR: Vec3 = vec3(0.47, 0.78, 0.2);

#[derive(Debug, Copy, Clone)]
enum Fs {
    A,              // main axis apex
    B,              // pinna apex
    F(f32),         // stem segment (carries current length)
    Leaf(f32, f32), // leaf quad (width, height)
    Turn(f32),
    Roll(f32),
    Push,
    Pop,
}

impl PartialEq for Fs {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Symbol for Fs {
    fn symbol_type(&self) -> SymbolType {
        match self {
            Fs::A | Fs::B | Fs::F(_) => SymbolType::NonTerminal,
            _ => SymbolType::Terminal,
        }
    }
}

impl From<Fs> for Action {
    fn from(s: Fs) -> Self {
        match s {
            Fs::F(l) => Action::Branch(l, 0.025),
            Fs::Leaf(w, h) => Action::Leaf(w, h),
            Fs::Turn(a) => Action::Turn(a),
            Fs::Roll(a) => Action::Roll(a),
            Fs::Push => Action::Push,
            Fs::Pop => Action::Pop,
            _ => Action::Nop,
        }
    }
}

pub struct FernPlant {
    age: u32,
    pub colour: Vec3,
    dirty: bool,
    cached_actions: Vec<Action>,
}

impl FernPlant {
    pub fn new(age: u32) -> Self {
        let actions = generate(age, DEFAULT_COLOUR);
        Self {
            age,
            colour: DEFAULT_COLOUR,
            dirty: false,
            cached_actions: actions,
        }
    }
}

impl Plant for FernPlant {
    fn plant_type(&self) -> PlantType {
        PlantType::Fern
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
        self.colour
    }

    fn clone_boxed(&self) -> Box<dyn Plant> {
        let mut p = Self::new(self.age);
        p.colour = self.colour;
        Box::new(p)
    }

    fn actions(&mut self) -> &[Action] {
        if self.dirty {
            self.cached_actions = generate(self.age, self.colour);
            self.dirty = false;
        }
        &self.cached_actions
    }
}

fn generate(age: u32, _colour: Vec3) -> Vec<Action> {
    use Fs::*;
    let a = 22.0f32.to_radians(); // pinna angle
    let b = 35.0f32.to_radians(); // leaf angle within pinna
    let f_init = 0.07f32; // initial segment length (new growth)
    let f_max = 0.22f32; // maximum segment length (mature growth)

    // F(l): each iteration the existing segment grows toward f_max; new segments start at f_init.
    // This gives natural tapering: basal segments are long, terminal ones short.
    let rule_f = Rule::Parametric(F(0.0), &move |s, out| {
        let &Fs::F(l) = s else { unreachable!() };
        out.extend([F((l * 1.2).min(f_max))]);
        true
    });

    let rule_a = Rule::Normal(A, &[
        F(f_init),
        Push,
        Turn(a),
        B,
        Pop,
        Push,
        Turn(-a),
        B,
        Pop,
        Roll(GOLDEN_ANGLE),
        A,
    ]);

    let rule_b = Rule::Normal(B, &[
        F(f_init),
        F(f_init),
        Push,
        Turn(b),
        Leaf(0.06, 0.10),
        Pop,
        Push,
        Turn(-b),
        Leaf(0.06, 0.10),
        Pop,
    ]);

    let mut lsystem: LSystem<Fs> = LSystem::new(&[A], vec![rule_f, rule_a, rule_b]);
    lsystem.evolve(age as usize);
    lsystem.current().iter().map(|s| Action::from(*s)).collect()
}
