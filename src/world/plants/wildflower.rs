use glam::{Vec3, vec3};

use crate::{
    settings::PlantType,
    util::{
        lsystem::{LSystem, Rule, Symbol, SymbolType},
        rng,
        turtle::Action,
    },
    world::plants::Plant,
};

const GOLDEN_ANGLE: f32 = 2.399655;

// ── Wildflower symbol type ──

#[derive(Debug, Copy, Clone)]
enum Wf {
    P,
    A(f32),
    I(f32),
    B,
    F,
    Right(f32),
    Left(f32),
    Roll(f32),
    Colour(Vec3),
    Leaf(f32, f32),
    Push,
    Pop,
}

impl PartialEq for Wf {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Symbol for Wf {
    fn symbol_type(&self) -> SymbolType {
        match self {
            Wf::F | Wf::P | Wf::A(_) | Wf::I(_) | Wf::B => SymbolType::NonTerminal,
            _ => SymbolType::Terminal,
        }
    }
}

impl From<Wf> for Action {
    fn from(val: Wf) -> Self {
        match val {
            Wf::F => Action::Branch(0.1, 0.02),
            Wf::Right(a) => Action::Turn(a.to_radians()),
            Wf::Left(a) => Action::Turn(-a.to_radians()),
            Wf::Roll(a) => Action::Roll(a),
            Wf::Colour(rgb) => Action::Colour(rgb),
            Wf::Leaf(w, h) => Action::Leaf(w * 0.01, h * 0.01),
            Wf::Push => Action::Push,
            Wf::Pop => Action::Pop,
            _ => Action::Nop,
        }
    }
}

// ── WildflowerPlant ──

pub struct WildflowerPlant {
    age: u32,
    pub flower_colour: [f32; 3],
    pub leaf_colour: [f32; 3],
    dirty: bool,
    cached_actions: Vec<Action>,
}

impl WildflowerPlant {
    pub fn new(age: u32) -> Self {
        let flower = [1.0, 0.0, 1.0];
        let leaf = [0.39, 0.98, 0.0];
        let actions = generate(age, Vec3::from(flower), Vec3::from(leaf));
        Self {
            age,
            flower_colour: flower,
            leaf_colour: leaf,
            dirty: false,
            cached_actions: actions,
        }
    }
}

impl Plant for WildflowerPlant {
    fn plant_type(&self) -> PlantType {
        PlantType::Wildflower
    }

    fn age(&self) -> u32 {
        self.age
    }

    fn max_age(&self) -> u32 {
        20
    }

    fn set_age(&mut self, age: u32) {
        if self.age != age {
            self.age = age;
            self.dirty = true;
        }
    }

    fn colour(&self) -> Vec3 {
        vec3(0.2, 0.71, 0.2)
    }

    fn clone_boxed(&self) -> Box<dyn Plant> {
        let mut plant = Self::new(self.age);
        plant.flower_colour = self.flower_colour;
        plant.leaf_colour = self.leaf_colour;
        plant.dirty = true;
        Box::new(plant)
    }

    fn actions(&mut self) -> &[Action] {
        if self.dirty {
            self.cached_actions = generate(
                self.age,
                Vec3::from(self.flower_colour),
                Vec3::from(self.leaf_colour),
            );
            self.dirty = false;
        }
        &self.cached_actions
    }
}

fn generate(age: u32, flower_rgb: Vec3, leaf_rgb: Vec3) -> Vec<Action> {
    use Wf::*;

    let axiom = [P];

    let rule_p = Rule::Normal(P, &[Push, A(rng::random_range(0.0, 10.0f32)), Pop]);

    // A(a) and I(a) carry their param inline → Rule::Parametric receives &Wf
    let rule_a = Rule::Parametric(A(0.0), &move |s, out| {
        let &A(a) = s else { unreachable!() };
        let wobble = if a > 0.0 {
            rng::random_range(-a, a)
        } else {
            0.0
        };
        out.extend([
            F,
            F,
            F,
            I(40.0),
            Right(wobble),
            Roll(GOLDEN_ANGLE),
            A(a + 1.0),
            Push,
            Colour(flower_rgb),
            Leaf(4.0, 4.0),
            Pop,
        ]);
        true
    });

    let rule_i = Rule::Parametric(I(0.0), &move |s, out| {
        let &I(a) = s else { unreachable!() };
        out.extend([
            Push,
            Left(a),
            B,
            Pop,
            Roll(GOLDEN_ANGLE),
            Push,
            Right(a),
            B,
            Pop,
        ]);
        true
    });

    let rule_b_grow =
        Rule::Stochastic(B, 0.5, &[F, B, Push, Colour(leaf_rgb), Leaf(2.0, 2.0), Pop]);
    let rule_b_stop = Rule::Normal(B, &[I(rng::random_range(0.0, 10.0))]);

    let mut lsystem: LSystem<Wf> = LSystem::new(&axiom, vec![
        rule_p,
        rule_a,
        rule_i,
        rule_b_grow,
        rule_b_stop,
    ]);

    lsystem.evolve(age as usize);
    lsystem.current().iter().map(|s| Action::from(*s)).collect()
}
