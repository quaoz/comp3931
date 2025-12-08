use std::f32::consts::{FRAC_PI_2, PI};

use glam::{Vec3, vec3};
use wgpu::{Buffer, Queue};
use winit::keyboard::KeyCode;

use crate::{
    turtle::turtle::{Action, Line, Turtle},
    util::lsystem::{LSystem, Rule, Symbol, SymbolType},
};

const RED: Vec3 = vec3(255.0, 0.0, 0.0);
const GREEN: Vec3 = vec3(0.0, 255.0, 0.0);
const CYAN: Vec3 = vec3(0.0, 255.0, 255.0);
const CYAN2: Vec3 = vec3(1.0, 255.0, 255.0);
const MAGENTA: Vec3 = vec3(255.0, 0.0, 255.0);

const HALF: f32 = FRAC_PI_2;
const FULL: f32 = PI;
const SIZE: f32 = 10.0;

#[derive(Debug, PartialEq, Eq)]
enum Scenes {
    Cube,
    Spiralosaurus,
    Braidwork,
    Dragoncurve,
}

#[derive(Debug)]
pub struct SceneController {
    scene: Scenes,
    dirty_scene: bool,
    scene_len: u32,
    scene_iter: u32,
}

impl SceneController {
    pub fn new() -> Self {
        Self {
            scene: Scenes::Cube,
            dirty_scene: true,
            scene_len: 0,
            scene_iter: 5,
        }
    }

    pub fn handle_keyboard(&mut self, key: KeyCode, pressed: bool) -> bool {
        if pressed {
            match key {
                KeyCode::Digit1 => {
                    self.scene = Scenes::Cube;
                    self.scene_iter = 5;
                }
                KeyCode::Digit2 => {
                    self.scene = Scenes::Spiralosaurus;
                    self.scene_iter = 5;
                }
                KeyCode::Digit3 => {
                    self.scene = Scenes::Braidwork;
                    self.scene_iter = 5;
                }
                KeyCode::Digit4 => {
                    self.scene = Scenes::Dragoncurve;
                    self.scene_iter = 5;
                }
                KeyCode::ArrowUp => self.scene_iter += 1,
                KeyCode::ArrowDown => self.scene_iter -= 1,
                _ => return false,
            };

            self.dirty_scene = true;
            return true;
        }
        false
    }

    pub fn set_scene(&mut self, queue: &Queue, buffer: &Buffer) -> u32 {
        if !self.dirty_scene {
            return self.scene_len;
        }

        let mut turtle = Turtle::new(vec3(-SIZE / 2.0, -SIZE / 2.0, SIZE), RED);

        match self.scene {
            Scenes::Cube => {
                let a = vec![Action::Travel(SIZE), Action::Turn(HALF)].repeat(3);
                let b = vec![
                    Action::Push,
                    Action::Turn(HALF),
                    Action::Colour(GREEN),
                    Action::Travel(SIZE),
                    Action::Pop,
                    Action::Turn(FULL),
                    Action::Colour(CYAN),
                    Action::Travel(SIZE),
                ]
                .repeat(3);
                let c = [
                    a,
                    vec![
                        Action::Travel(SIZE),
                        Action::Roll(HALF),
                        Action::Turn(HALF),
                        Action::Colour(GREEN),
                        Action::Travel(SIZE),
                        Action::Turn(HALF),
                        Action::Colour(CYAN),
                        Action::Travel(SIZE),
                    ],
                    b,
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<Action>>();

                turtle.do_actions(&c);
            }
            Scenes::Spiralosaurus => {
                // let scene = &turtle.do_segments(&[
                // t(4.0),
                // t(4.0),
                // l(9.0),
                // l(9.0),
                // t(4.0),
                // t(4.0),
                // r(3.0),
                // r(3.0),
                // r(3.0),
                // r(3.0),
                // r(3.0),
                // r(3.0),
                // t(4.0),
                // t(4.0),
                // l(9.0),
                // l(9.0),
                // t(4.0),
                // t(4.0),
                // r(3.0),
                // r(3.0),
                // r(3.0),
                // r(3.0),
                // r(3.0),
                // r(3.0),
                // t(4.0),
                // t(4.0),
                // l(9.0),
                // l(9.0),
                // t(4.0),
                // t(4.0),
                // r(3.0),
                // r(3.0),
                // r(3.0),
                // r(3.0),
                // r(3.0),
                // r(3.0),
                // ]);
                //
                // queue.write_buffer(buffer, 0, bytemuck::cast_slice(scene));
                // self.scene_len = scene.len() as u32
            }
            Scenes::Braidwork => {
                // let scene = &turtle.do_segments(&[
                // l(1.0),
                // r(5.0),
                // r(6.0),
                // r(6.0),
                // l(3.0),
                // r(1.0),
                // l(5.0),
                // l(6.0),
                // l(6.0),
                // r(3.0),
                // l(1.0),
                // r(5.0),
                // r(6.0),
                // r(6.0),
                // l(3.0),
                // r(1.0),
                // l(5.0),
                // l(6.0),
                // l(6.0),
                // r(3.0),
                // l(1.0),
                // r(5.0),
                // r(6.0),
                // r(6.0),
                // l(3.0),
                // r(1.0),
                // l(5.0),
                // l(6.0),
                // l(6.0),
                // r(3.0),
                // ]);
                //
                // queue.write_buffer(buffer, 0, bytemuck::cast_slice(scene));
                // self.scene_len = scene.len() as u32
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

                impl Into<Action> for Curve {
                    fn into(self) -> Action {
                        match self {
                            Curve::F => Action::Travel(1.0),
                            Curve::G => Action::Travel(1.0),
                            Curve::X => Action::Turn(HALF),
                            Curve::Y => Action::Turn(-HALF),
                        }
                    }
                }

                let mut lsystem = LSystem::new(&[Curve::F], vec![
                    Rule::Normal(Curve::F, &[Curve::F, Curve::X, Curve::G]),
                    Rule::Normal(Curve::G, &[Curve::F, Curve::Y, Curve::G]),
                ]);
                lsystem.evolve(self.scene_iter as usize);

                turtle.do_actions(lsystem.current());

                // queue.write_buffer(buffer, 0, bytemuck::cast_slice(&lines));
                // self.scene_len = lines.len() as u32;
            }
        }

        let mut lines = Vec::new();
        let mut last_path_idx = 0;
        for (jump, path_idx, colour_idx) in turtle.path_indicies.iter().skip(1) {
            if !jump {
                lines.push(Line {
                    start: turtle.path_buf[last_path_idx],
                    end: turtle.path_buf[*path_idx],
                    colour: turtle.colour_buf[*colour_idx],
                });
            };

            last_path_idx = *path_idx;
        }

        queue.write_buffer(buffer, 0, bytemuck::cast_slice(&lines));

        self.scene_len = lines.len() as u32;
        self.scene_len
    }
}
