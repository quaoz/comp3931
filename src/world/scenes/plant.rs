use crate::util::{
    lsystem::{LSystem, Rule, Symbol, SymbolType},
    turtle::Action,
};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Plant {
    X,
    F,
    Right,
    Left,
    Push,
    Pop,
}

impl Symbol for Plant {
    fn symbol_type(&self) -> SymbolType {
        match self {
            Plant::X | Plant::F => SymbolType::NonTerminal,
            _ => SymbolType::Terminal,
        }
    }
}

impl From<Plant> for Action {
    fn from(val: Plant) -> Self {
        match val {
            Plant::X => Action::Nop,
            Plant::F => Action::Branch(0.1, 0.02),
            Plant::Right => Action::Turn(25f32.to_radians()),
            Plant::Left => Action::Turn(-25f32.to_radians()),
            Plant::Push => Action::Push,
            Plant::Pop => Action::Pop,
        }
    }
}

pub fn actions(scene_iter: u32) -> Vec<Plant> {
    let mut lsystem = LSystem::new(&[Plant::Right, Plant::X], vec![
        Rule::Normal(Plant::X, &[
            Plant::F,
            Plant::Left,
            Plant::Push,
            Plant::Push,
            Plant::X,
            Plant::Pop,
            Plant::Right,
            Plant::X,
            Plant::Pop,
            Plant::Right,
            Plant::F,
            Plant::Push,
            Plant::Right,
            Plant::F,
            Plant::X,
            Plant::Pop,
            Plant::Left,
            Plant::X,
        ]),
        Rule::Normal(Plant::F, &[Plant::F, Plant::F]),
    ]);

    lsystem.evolve(scene_iter as usize);
    lsystem.current().to_owned()
}
