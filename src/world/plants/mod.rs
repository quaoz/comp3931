pub mod bush;
pub mod capsella;
pub mod carrot;
pub mod fern;
pub mod lychnis;
pub mod mint;
pub mod mycelis;
pub mod tree;
pub mod wildflower;

use glam::Vec3;

use crate::{
    settings::PlantType,
    util::{
        lsystem::{Symbol, SymbolType},
        turtle::Action,
    },
};

// ── Shared symbol type used by tree and bush ──

#[derive(Debug, PartialEq, Copy, Clone)]
pub(crate) enum PlantSymbol {
    X,
    F,
    Right(f32),
    Left(f32),
    Roll(f32),
    Push,
    Pop,
}

impl Symbol for PlantSymbol {
    fn symbol_type(&self) -> SymbolType {
        match self {
            PlantSymbol::X | PlantSymbol::F => SymbolType::NonTerminal,
            _ => SymbolType::Terminal,
        }
    }
}

impl From<PlantSymbol> for Action {
    fn from(val: PlantSymbol) -> Self {
        match val {
            PlantSymbol::X => Action::Nop,
            PlantSymbol::F => Action::Branch(0.1, 0.02),
            PlantSymbol::Right(a) => Action::Turn(a),
            PlantSymbol::Left(a) => Action::Turn(-a),
            PlantSymbol::Roll(a) => Action::Roll(a),
            PlantSymbol::Push => Action::Push,
            PlantSymbol::Pop => Action::Pop,
        }
    }
}

// ── Plant trait ──

pub trait Plant: Send {
    fn plant_type(&self) -> PlantType;
    fn age(&self) -> u32;
    fn max_age(&self) -> u32;
    fn set_age(&mut self, age: u32);
    fn actions(&mut self) -> &[Action];
    fn colour(&self) -> Vec3;
    fn clone_boxed(&self) -> Box<dyn Plant>;
}

// ── PlantInstance wrapper ──

pub struct PlantInstance {
    pub position: [f32; 3],
    pub scale: f32,
    pub rotation: f32,
    pub base_age: u32,
    pub plant: Box<dyn Plant>,
}

impl PlantInstance {
    pub fn new(plant_type: PlantType, age: u32) -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            scale: 1.0,
            rotation: 0.0,
            base_age: age,
            plant: create_plant(plant_type, age),
        }
    }

    pub fn with_transform(mut self, position: [f32; 3], scale: f32, rotation: f32) -> Self {
        self.position = position;
        self.scale = scale;
        self.rotation = rotation;
        self
    }
}

impl Clone for PlantInstance {
    fn clone(&self) -> Self {
        Self {
            position: self.position,
            scale: self.scale,
            rotation: self.rotation,
            base_age: self.base_age,
            plant: self.plant.clone_boxed(),
        }
    }
}

impl std::fmt::Debug for PlantInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlantInstance")
            .field("plant_type", &self.plant.plant_type())
            .field("base_age", &self.base_age)
            .field("position", &self.position)
            .field("scale", &self.scale)
            .field("rotation", &self.rotation)
            .finish()
    }
}

fn create_plant(plant_type: PlantType, age: u32) -> Box<dyn Plant> {
    match plant_type {
        PlantType::Tree => Box::new(tree::TreePlant::new(age)),
        PlantType::Bush => Box::new(bush::BushPlant::new(age)),
        PlantType::Fern => Box::new(fern::FernPlant::new(age)),
        PlantType::Wildflower => Box::new(wildflower::WildflowerPlant::new(age)),
        PlantType::Capsella => Box::new(capsella::CapsellaPant::new(age)),
        PlantType::Mint => Box::new(mint::MintPlant::new(age)),
        PlantType::Lychnis => Box::new(lychnis::LychnisPant::new(age)),
        PlantType::Mycelis => Box::new(mycelis::MycelisPlant::new(age)),
        PlantType::Carrot => Box::new(carrot::CarrotPlant::new(age)),
    }
}
