use std::f32::consts::FRAC_PI_2;

use glam::{Vec3, vec3};
use wgpu::{Buffer, Queue};
use winit::keyboard::KeyCode;

use crate::util::turtle::Turtle;

pub struct SceneBuffers<'a> {
    pub queue: &'a Queue,
    pub line_vertex: &'a Buffer,
    pub line_color: &'a Buffer,
    pub line_index: &'a Buffer,
    pub mesh_vertex: &'a Buffer,
    pub mesh_normal: &'a Buffer,
    pub mesh_color: &'a Buffer,
    pub mesh_index: &'a Buffer,
}

const RED: Vec3 = vec3(255.0, 0.0, 0.0);
const GREEN: Vec3 = vec3(0.0, 255.0, 0.0);
const CYAN: Vec3 = vec3(0.0, 255.0, 255.0);
const MAGENTA: Vec3 = vec3(255.0, 0.0, 255.0);

mod dragoncurve;
mod peano;
mod plant;
mod sierpinski;
mod spiralosaurus;

#[derive(Debug, PartialEq, Eq)]
enum Scenes {
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
    mesh_index_count: u32,
    scene_iter: u32,
    scene_scale: f32,
}

impl Default for SceneController {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneController {
    pub fn new() -> Self {
        Self {
            scene: Scenes::Plant,
            turtle: Turtle::new(Vec3::ZERO, RED),
            dirty_scene: true,
            scene_len: Vec::new(),
            mesh_index_count: 0,
            scene_iter: 5,
            scene_scale: 10.0,
        }
    }

    pub fn handle_keyboard(&mut self, key: KeyCode, pressed: bool) -> bool {
        if pressed {
            match key {
                KeyCode::Digit1 => {
                    self.scene = Scenes::Spiralosaurus;
                    self.scene_iter = 5;
                    self.scene_scale = 10.0;
                }
                KeyCode::Digit2 => {
                    self.scene = Scenes::Dragoncurve;
                    self.scene_iter = 5;
                    self.scene_scale = 10.0;
                }
                KeyCode::Digit3 => {
                    self.scene = Scenes::Sierpinski;
                    self.scene_iter = 5;
                    self.scene_scale = 10.0;
                }
                KeyCode::Digit4 => {
                    self.scene = Scenes::Plant;
                    self.scene_iter = 5;
                    self.scene_scale = 10.0;
                }
                KeyCode::Digit5 => {
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

    pub fn set_scene(&mut self, buffers: &SceneBuffers) -> (Vec<(u32, u32)>, u32) {
        if !self.dirty_scene {
            return (self.scene_len.clone(), self.mesh_index_count);
        }

        self.turtle.reset(Vec3::ZERO, RED);
        self.turtle.set_scale(self.scene_scale);

        match self.scene {
            Scenes::Spiralosaurus => {
                let actions = spiralosaurus::actions();
                self.turtle.do_actions(&actions);
            }
            Scenes::Dragoncurve => {
                let actions = dragoncurve::actions(self.scene_iter);
                self.turtle.do_actions(actions.as_slice());
            }
            Scenes::Sierpinski => {
                let actions = sierpinski::actions(self.scene_iter);
                self.turtle.set_colour(CYAN);
                self.turtle.do_actions(actions.as_slice());
            }
            Scenes::Plant => {
                let actions = plant::actions(self.scene_iter);
                self.turtle.roll(FRAC_PI_2);
                self.turtle.turn(FRAC_PI_2);
                self.turtle.set_colour(GREEN);
                self.turtle.do_actions(actions.as_slice());
            }
            Scenes::Peano => {
                let actions = peano::actions(self.scene_iter);
                self.turtle.set_colour(MAGENTA);
                self.turtle.do_actions(actions.as_slice());
            }
        }

        self.dirty_scene = false;
        self.scene_len = self.turtle.write_to_buffers(
            buffers.queue,
            buffers.line_vertex,
            buffers.line_color,
            buffers.line_index,
        );
        self.mesh_index_count = self.turtle.write_mesh_to_buffers(
            buffers.queue,
            buffers.mesh_vertex,
            buffers.mesh_normal,
            buffers.mesh_color,
            buffers.mesh_index,
        );
        (self.scene_len.clone(), self.mesh_index_count)
    }
}
