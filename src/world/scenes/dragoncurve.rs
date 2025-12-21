use std::f32::consts::FRAC_PI_2;

use crate::util::{
    lsystem::{LSystem, Rule, Symbol, SymbolType},
    turtle::Action,
};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Curve {
    F,
    G,
    X,
    Y,
}

impl Symbol for Curve {
    fn symbol_type(&self) -> SymbolType {
        match self {
            Curve::F | Curve::G => SymbolType::NonTerminal,
            _ => SymbolType::Terminal,
        }
    }
}

impl From<Curve> for Action {
    fn from(val: Curve) -> Self {
        match val {
            Curve::F => Action::Travel(0.1),
            Curve::G => Action::Travel(0.1),
            Curve::X => Action::Turn(FRAC_PI_2),
            Curve::Y => Action::Turn(-FRAC_PI_2),
        }
    }
}

pub fn actions(scene_iter: u32) -> Vec<Curve> {
    let mut lsystem = LSystem::new(&[Curve::F], vec![
        Rule::Normal(Curve::F, &[Curve::F, Curve::X, Curve::G]),
        Rule::Normal(Curve::G, &[Curve::F, Curve::Y, Curve::G]),
    ]);

    lsystem.evolve(scene_iter as usize);
    lsystem.current().to_owned()
}
