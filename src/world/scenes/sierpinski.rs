use std::f32::consts::FRAC_PI_3;

use crate::util::{
    lsystem::{LSystem, Rule, Symbol, SymbolType},
    turtle::Action,
};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Sierpinski {
    F,
    G,
    X,
    Y,
}

impl Symbol for Sierpinski {
    fn symbol_type(&self) -> SymbolType {
        match self {
            Sierpinski::F | Sierpinski::G => SymbolType::NonTerminal,
            _ => SymbolType::Terminal,
        }
    }
}

impl From<Sierpinski> for Action {
    fn from(val: Sierpinski) -> Self {
        match val {
            Sierpinski::F => Action::Travel(0.1),
            Sierpinski::G => Action::Travel(0.1),
            Sierpinski::X => Action::Turn(FRAC_PI_3 * 2.0),
            Sierpinski::Y => Action::Turn(-FRAC_PI_3 * 2.0),
        }
    }
}

pub fn actions(scene_iter: u32) -> Vec<Sierpinski> {
    let mut lsystem = LSystem::new(
        &[
            Sierpinski::F,
            Sierpinski::Y,
            Sierpinski::G,
            Sierpinski::Y,
            Sierpinski::G,
        ],
        vec![
            Rule::Normal(Sierpinski::F, &[
                Sierpinski::F,
                Sierpinski::Y,
                Sierpinski::G,
                Sierpinski::X,
                Sierpinski::F,
                Sierpinski::X,
                Sierpinski::G,
                Sierpinski::Y,
                Sierpinski::F,
            ]),
            Rule::Normal(Sierpinski::G, &[Sierpinski::G, Sierpinski::G]),
        ],
    );

    lsystem.evolve(scene_iter as usize);
    lsystem.current().to_owned()
}
