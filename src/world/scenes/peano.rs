use std::f32::consts::FRAC_PI_2;

use crate::util::{
    lsystem::{LSystem, Rule, Symbol, SymbolType},
    turtle::Action,
};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Peano {
    X,
    Y,
    F,
    Right,
    Left,
}

impl Symbol for Peano {
    fn symbol_type(&self) -> SymbolType {
        match self {
            Peano::X | Peano::Y => SymbolType::NonTerminal,
            _ => SymbolType::Terminal,
        }
    }
}

impl From<Peano> for Action {
    fn from(val: Peano) -> Self {
        match val {
            Peano::F => Action::Travel(0.1),
            Peano::Right => Action::Turn(FRAC_PI_2),
            Peano::Left => Action::Turn(-FRAC_PI_2),
            _ => Action::Nop,
        }
    }
}

pub fn actions(scene_iter: u32) -> Vec<Peano> {
    let mut lsystem = LSystem::new(&[Peano::X], vec![
        Rule::Normal(Peano::X, &[
            Peano::X,
            Peano::F,
            Peano::Y,
            Peano::F,
            Peano::X,
            Peano::Right,
            Peano::F,
            Peano::Right,
            Peano::Y,
            Peano::F,
            Peano::X,
            Peano::F,
            Peano::Y,
            Peano::Left,
            Peano::F,
            Peano::Left,
            Peano::X,
            Peano::F,
            Peano::Y,
            Peano::F,
            Peano::X,
        ]),
        Rule::Normal(Peano::Y, &[
            Peano::Y,
            Peano::F,
            Peano::X,
            Peano::F,
            Peano::Y,
            Peano::Left,
            Peano::F,
            Peano::Left,
            Peano::X,
            Peano::F,
            Peano::Y,
            Peano::F,
            Peano::X,
            Peano::Right,
            Peano::F,
            Peano::Right,
            Peano::Y,
            Peano::F,
            Peano::X,
            Peano::F,
            Peano::Y,
        ]),
    ]);

    lsystem.evolve(scene_iter as usize);
    lsystem.current().to_owned()
}
