use std::f32::consts::FRAC_PI_2;

use crate::{
    util::turtle::Action,
    world::scenes::{CYAN, GREEN, RED},
};

pub fn actions() -> Vec<Action> {
    [
        [
            Action::Colour(RED),
            Action::Travel(0.4),
            Action::Turn(FRAC_PI_2),
        ]
        .repeat(2),
        [
            Action::Colour(GREEN),
            Action::Travel(0.9),
            Action::Roll(-FRAC_PI_2),
            Action::Turn(FRAC_PI_2),
        ]
        .repeat(2),
        [
            Action::Colour(RED),
            Action::Travel(0.4),
            Action::Turn(FRAC_PI_2),
        ]
        .repeat(2),
        [
            Action::Colour(CYAN),
            Action::Travel(0.3),
            Action::Roll(FRAC_PI_2),
            Action::Turn(FRAC_PI_2),
        ]
        .repeat(6),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<Action>>()
    .repeat(3)
}
