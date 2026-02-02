use glam::{Vec3, vec3};

use super::PlantSymbol::*;
use crate::{
    settings::PlantType,
    util::{
        lsystem::{LSystem, Rule},
        turtle::Action,
    },
    world::plants::Plant,
};

const GOLDEN_ANGLE: f32 = 2.399655;
const DEFAULT_COLOUR: Vec3 = vec3(0.71, 0.31, 0.16);

pub struct BushPlant {
    age: u32,
    pub colour: Vec3,
    dirty: bool,
    cached_actions: Vec<Action>,
}

impl BushPlant {
    pub fn new(age: u32) -> Self {
        let actions = generate(age);
        Self {
            age,
            colour: DEFAULT_COLOUR,
            dirty: false,
            cached_actions: actions,
        }
    }
}

impl Plant for BushPlant {
    fn plant_type(&self) -> PlantType {
        PlantType::Bush
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
        self.colour
    }

    fn clone_boxed(&self) -> Box<dyn Plant> {
        let mut p = Self::new(self.age);
        p.colour = self.colour;
        Box::new(p)
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
    let a = 30.0f32.to_radians();

    let axiom = [Right(a), X];
    let rule = Rule::Normal(X, &[
        F,
        Push,
        Left(a),
        X,
        Pop,
        Roll(GOLDEN_ANGLE),
        F,
        Push,
        Right(a),
        X,
        Pop,
        Roll(GOLDEN_ANGLE),
        Push,
        Left(a),
        F,
        X,
        Pop,
    ]);

    let mut lsystem = LSystem::new(&axiom, vec![rule, Rule::Normal(F, &[F, F])]);
    lsystem.evolve(age as usize);
    lsystem.current().iter().map(|s| Action::from(*s)).collect()
}
