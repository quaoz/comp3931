use std::{f32::consts::PI, time::Duration};

use glam::{Mat4, Vec3, vec3};
use winit::{event::MouseScrollDelta, keyboard::KeyCode};

use crate::{
    settings::Settings,
    world::{
        camera::{Camera, CameraController, Projection},
        scenes::SceneController,
    },
};

pub mod camera;
pub mod plants;
pub mod scenes;

#[derive(Debug)]
pub struct World {
    pub camera: Camera,
    projection: Projection,
    camera_controller: CameraController,
    scene_controller: SceneController,
}

impl World {
    pub fn new(surface_width: u32, surface_height: u32) -> Self {
        let camera = Camera::new(vec3(-50.0, 50.0, -50.0), std::f32::consts::FRAC_PI_2, 0.0);
        let camera_controller = CameraController::new(25.0, 0.001);
        let projection = Projection::new(surface_width, surface_height, PI * 0.25, 0.05, 5000.0);
        let scene_controller = SceneController::new();

        Self {
            camera,
            projection,
            camera_controller,
            scene_controller,
        }
    }

    pub fn reset_camera(&mut self) {
        self.camera = Camera::new(vec3(-50.0, 50.0, -50.0), std::f32::consts::FRAC_PI_2, 0.0);
    }

    pub fn camera_position(&self) -> Vec3 {
        self.camera.camera_position()
    }

    pub fn apply_settings(&mut self, settings: &Settings) {
        self.camera_controller.set_speed(settings.camera.speed);
        self.camera_controller
            .set_sensitivity(settings.camera.sensitivity);
        self.projection.set_fov(settings.display.fov);
    }

    pub fn handle_mouse_motion(&mut self, dx: f64, dy: f64) {
        self.camera_controller.process_mouse(dx, dy);
    }

    pub fn handle_mouse_scroll(&mut self, delta: &MouseScrollDelta) {
        self.camera_controller.process_mouse_scroll(delta);
    }

    pub fn handle_keyboard(&mut self, key: KeyCode, pressed: bool) {
        self.camera_controller.process_keyboard(key, pressed);
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.projection.resize(width, height);
    }

    pub fn update(&mut self, dt: Duration) {
        self.camera_controller.update_camera(&mut self.camera, dt);
    }

    pub fn view_proj(&self) -> Mat4 {
        self.projection.calc_matrix() * self.camera.calc_matrix()
    }

    pub fn scene_controller(&mut self) -> &mut SceneController {
        &mut self.scene_controller
    }
}
