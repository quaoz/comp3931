/// Carrot plant inspired by "The Algorithmic Beauty of Plants" fig 3.23, pg 96.
/// Models a compound 5-rayed umbel with secondary rays and terminal florets.
/// The feathery compound foliage is also modelled with pinnate branching.
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
const STEM_COLOUR: Vec3 = vec3(0.4, 0.65, 0.2);
const FLOWER_COLOUR: Vec3 = vec3(0.95, 0.95, 0.95);
const LEAF_COLOUR: Vec3 = vec3(0.35, 0.75, 0.2);

#[derive(Debug, Copy, Clone)]
enum Cr {
    U,      // umbel (5-rayed radial structure)
    R,      // primary ray
    S,      // secondary ray
    L,      // leaf (on stem)
    K,      // floret (terminal)
    I(f32), // internode (carries current length)
    Turn(f32),
    Roll(f32),
    Colour(Vec3),
    Push,
    Pop,
}

impl PartialEq for Cr {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Symbol for Cr {
    fn symbol_type(&self) -> SymbolType {
        match self {
            Cr::U | Cr::R | Cr::S | Cr::L | Cr::K | Cr::I(_) => SymbolType::NonTerminal,
            _ => SymbolType::Terminal,
        }
    }
}

impl From<Cr> for Action {
    fn from(s: Cr) -> Self {
        match s {
            Cr::I(l) => Action::Branch(l, 0.022),
            Cr::L => Action::Leaf(0.06, 0.10),
            Cr::K => Action::Leaf(0.03, 0.03),
            Cr::Turn(a) => Action::Turn(a),
            Cr::Roll(a) => Action::Roll(a),
            Cr::Colour(c) => Action::Colour(c),
            Cr::Push => Action::Push,
            Cr::Pop => Action::Pop,
            _ => Action::Nop,
        }
    }
}

pub struct CarrotPlant {
    age: u32,
    dirty: bool,
    cached_actions: Vec<Action>,
}

impl CarrotPlant {
    pub fn new(age: u32) -> Self {
        let actions = generate(age);
        Self {
            age,
            dirty: false,
            cached_actions: actions,
        }
    }
}

impl Plant for CarrotPlant {
    fn plant_type(&self) -> PlantType {
        PlantType::Carrot
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
    use Cr::*;
    let ray_angle = 40.0f32.to_radians(); // angle of primary rays from vertical
    let sub_angle = 30.0f32.to_radians(); // angle of secondary rays
    let ray_roll = std::f32::consts::TAU / 5.0; // 72° = 2π/5 for 5-fold symmetry
    let leaf_angle = 35.0f32.to_radians();
    let i_init = 0.05f32;
    let i_max = 0.16f32;

    let rule_i = Rule::Parametric(I(0.0), &move |s, out| {
        let &Cr::I(l) = s else { unreachable!() };
        out.extend([I((l * 1.2).min(i_max))]);
        true
    });

    // Main stem: grow upward, add pinnate leaves along stem, terminate in compound umbel
    let rule_u = Rule::Normal(U, &[
        // 5 primary rays at 72° intervals
        Push,
        Roll(0.0 * ray_roll),
        Turn(ray_angle),
        Colour(STEM_COLOUR),
        R,
        Pop,
        Push,
        Roll(1.0 * ray_roll),
        Turn(ray_angle),
        Colour(STEM_COLOUR),
        R,
        Pop,
        Push,
        Roll(2.0 * ray_roll),
        Turn(ray_angle),
        Colour(STEM_COLOUR),
        R,
        Pop,
        Push,
        Roll(3.0 * ray_roll),
        Turn(ray_angle),
        Colour(STEM_COLOUR),
        R,
        Pop,
        Push,
        Roll(4.0 * ray_roll),
        Turn(ray_angle),
        Colour(STEM_COLOUR),
        R,
        Pop,
    ]);

    // Primary ray: stem with secondary branches
    let rule_r = Rule::Normal(R, &[
        I(i_init),
        Push,
        Roll(GOLDEN_ANGLE),
        Turn(sub_angle),
        S,
        Pop,
        Push,
        Roll(-GOLDEN_ANGLE),
        Turn(-sub_angle),
        S,
        Pop,
        I(i_init),
        Push,
        Roll(GOLDEN_ANGLE * 2.0),
        Turn(sub_angle),
        S,
        Pop,
        I(i_init),
    ]);

    // Secondary ray: terminates in floret cluster
    let rule_s = Rule::Normal(S, &[
        I(i_init),
        Push,
        Roll(GOLDEN_ANGLE),
        Turn(sub_angle),
        Colour(FLOWER_COLOUR),
        K,
        Pop,
        Push,
        Roll(-GOLDEN_ANGLE),
        Turn(-sub_angle),
        Colour(FLOWER_COLOUR),
        K,
        Pop,
        Colour(FLOWER_COLOUR),
        K,
    ]);

    // Leaf on main stem
    let rule_l = Rule::Normal(L, &[
        Push,
        Turn(leaf_angle),
        Colour(LEAF_COLOUR),
        L,
        Pop,
        Push,
        Turn(-leaf_angle),
        Colour(LEAF_COLOUR),
        L,
        Pop,
    ]);

    // Plant: upright stem with alternating leaves leading to umbel at top
    let axiom = vec![
        Colour(STEM_COLOUR),
        I(i_init),
        Push,
        Roll(GOLDEN_ANGLE),
        Turn(leaf_angle),
        L,
        Pop,
        I(i_init),
        Push,
        Roll(-GOLDEN_ANGLE),
        Turn(leaf_angle),
        L,
        Pop,
        I(i_init),
        U,
    ];

    let mut lsystem: LSystem<Cr> =
        LSystem::new(&axiom, vec![rule_i, rule_u, rule_r, rule_s, rule_l]);
    lsystem.evolve(age as usize);
    lsystem.current().iter().map(|s| Action::from(*s)).collect()
}
