use std::f32::consts::{FRAC_PI_2, FRAC_PI_3, PI};

use glam::{Vec3, vec3};
use wgpu::{Buffer, Queue};
use winit::keyboard::KeyCode;

use crate::util::{
    lsystem::{LSystem, Rule, Symbol, SymbolType},
    turtle::{Action, Turtle},
};

const RED: Vec3 = vec3(255.0, 0.0, 0.0);
const GREEN: Vec3 = vec3(0.0, 255.0, 0.0);
const CYAN: Vec3 = vec3(0.0, 255.0, 255.0);
const MAGENTA: Vec3 = vec3(255.0, 0.0, 255.0);

#[derive(Debug, PartialEq, Eq)]
enum Scenes {
    Cube,
    Spiralosaurus,
    Dragoncurve,
    Sierpinski,
    Plant,
    Peano,
}

#[derive(Debug)]
pub struct SceneController {
    scene: Scenes,
    turtle: Turtle,
    dirty_scene: bool,
    scene_len: Vec<(u32, u32)>,
    scene_iter: u32,
    scene_scale: f32,
}

impl SceneController {
    pub fn new() -> Self {
        Self {
            scene: Scenes::Cube,
            turtle: Turtle::new(Vec3::ZERO, RED),
            dirty_scene: true,
            scene_len: Vec::new(),
            scene_iter: 5,
            scene_scale: 10.0,
        }
    }

    pub fn handle_keyboard(&mut self, key: KeyCode, pressed: bool) -> bool {
        if pressed {
            match key {
                KeyCode::Digit1 => {
                    self.scene = Scenes::Cube;
                    self.scene_iter = 5;
                    self.scene_scale = 10.0;
                }
                KeyCode::Digit2 => {
                    self.scene = Scenes::Spiralosaurus;
                    self.scene_iter = 5;
                    self.scene_scale = 10.0;
                }
                KeyCode::Digit3 => {
                    self.scene = Scenes::Dragoncurve;
                    self.scene_iter = 5;
                    self.scene_scale = 10.0;
                }
                KeyCode::Digit4 => {
                    self.scene = Scenes::Sierpinski;
                    self.scene_iter = 5;
                    self.scene_scale = 10.0;
                }
                KeyCode::Digit5 => {
                    self.scene = Scenes::Plant;
                    self.scene_iter = 5;
                    self.scene_scale = 10.0;
                }
                KeyCode::Digit6 => {
                    self.scene = Scenes::Peano;
                    self.scene_iter = 5;
                    self.scene_scale = 10.0;
                }
                KeyCode::ArrowUp => self.scene_scale *= 1.1,
                KeyCode::ArrowDown => self.scene_scale *= 0.9,
                KeyCode::ArrowRight => self.scene_iter += 1,
                KeyCode::ArrowLeft if self.scene_iter > 0 => self.scene_iter -= 1,
                _ => return false,
            };

            self.dirty_scene = true;
            return true;
        }
        false
    }

    pub fn set_scene(
        &mut self,
        queue: &Queue,
        vertex_buffer: &Buffer,
        color_buffer: &Buffer,
        index_buffer: &Buffer,
    ) -> Vec<(u32, u32)> {
        if !self.dirty_scene {
            return self.scene_len.clone();
        }

        self.turtle.reset(Vec3::ZERO, RED);
        self.turtle.set_scale(self.scene_scale);

        match self.scene {
            Scenes::Cube => {
                const SIZE: f32 = 10.0;
                let actions = [
                    [Action::Travel(SIZE), Action::Turn(FRAC_PI_2)].repeat(3),
                    vec![
                        Action::Travel(SIZE),
                        Action::Roll(FRAC_PI_2),
                        Action::Turn(FRAC_PI_2),
                        Action::Colour(GREEN),
                        Action::Travel(SIZE),
                        Action::Turn(FRAC_PI_2),
                        Action::Colour(CYAN),
                        Action::Travel(SIZE),
                    ],
                    [
                        Action::Push,
                        Action::Turn(FRAC_PI_2),
                        Action::Colour(GREEN),
                        Action::Travel(SIZE),
                        Action::Pop,
                        Action::Turn(PI),
                        Action::Colour(CYAN),
                        Action::Travel(SIZE),
                    ]
                    .repeat(3),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<Action>>();

                self.turtle.do_actions(&actions);
            }
            Scenes::Spiralosaurus => {
                let actions = [
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
                .repeat(3);

                self.turtle.do_actions(&actions);
            }
            Scenes::Dragoncurve => {
                #[derive(Debug, PartialEq, Copy, Clone)]
                enum Curve {
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

                let mut lsystem = LSystem::new(&[Curve::F], vec![
                    Rule::Normal(Curve::F, &[Curve::F, Curve::X, Curve::G]),
                    Rule::Normal(Curve::G, &[Curve::F, Curve::Y, Curve::G]),
                ]);

                lsystem.evolve(self.scene_iter as usize);
                self.turtle.do_actions(lsystem.current());
            }
            Scenes::Sierpinski => {
                #[derive(Debug, PartialEq, Copy, Clone)]
                enum Sierpinski {
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

                lsystem.evolve(self.scene_iter as usize);
                self.turtle.set_colour(CYAN);
                self.turtle.do_actions(lsystem.current());
            }
            Scenes::Plant => {
                #[derive(Debug, PartialEq, Copy, Clone)]
                enum Plant {
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
                            Plant::F => Action::Travel(0.1),
                            Plant::Right => Action::Turn(25f32.to_radians()),
                            Plant::Left => Action::Turn(-25f32.to_radians()),
                            Plant::Push => Action::Push,
                            Plant::Pop => Action::Pop,
                        }
                    }
                }

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

                lsystem.evolve(self.scene_iter as usize);
                self.turtle.roll(FRAC_PI_2);
                self.turtle.turn(FRAC_PI_2);
                self.turtle.set_colour(GREEN);
                self.turtle.do_actions(lsystem.current());
            }
            Scenes::Peano => {
                #[derive(Debug, PartialEq, Copy, Clone)]
                enum Peano {
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

                lsystem.evolve(self.scene_iter as usize);
                self.turtle.set_colour(MAGENTA);
                self.turtle.do_actions(lsystem.current());
            }
        }

        self.dirty_scene = false;
        self.scene_len =
            self.turtle
                .write_to_buffers(queue, vertex_buffer, color_buffer, index_buffer);
        self.scene_len.clone()
    }
}
