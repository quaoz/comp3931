use std::{f32::consts::PI, time::Duration};

use glam::{Mat4, Vec3, vec3};
use winit::{event::MouseScrollDelta, keyboard::KeyCode};

use crate::{
    settings::Settings,
    world::{
        camera::{Camera, CameraController, CameraMode, Projection},
        scenes::{SceneController, SceneStats},
    },
};

pub mod camera;
pub mod ecosystem;
pub mod plants;
pub mod scenes;

const DEFAULT_CAMERA_POS: Vec3 = vec3(-50.0, 50.0, -50.0);

pub struct CameraInfo {
    pub is_orbit: bool,
}

#[derive(Debug)]
pub struct World {
    pub camera: Camera,
    projection: Projection,
    camera_controller: CameraController,
    scene_controller: SceneController,
}

impl World {
    pub fn new(size: (u32, u32)) -> Self {
        let camera = Camera::new(DEFAULT_CAMERA_POS, std::f32::consts::FRAC_PI_2, 0.0);
        let camera_controller = CameraController::new(25.0, 0.001);
        let projection = Projection::new(size.0, size.1, PI * 0.25, 0.05, 5000.0);
        let scene_controller = SceneController::new();

        Self {
            camera,
            projection,
            camera_controller,
            scene_controller,
        }
    }

    pub fn reset_camera(&mut self) {
        self.camera = Camera::new(DEFAULT_CAMERA_POS, std::f32::consts::FRAC_PI_2, 0.0);
    }

    pub fn camera_position(&self) -> Vec3 {
        self.camera.camera_position()
    }

    pub fn camera_mode(&self) -> &CameraMode {
        &self.camera.mode
    }

    pub fn apply_settings(&mut self, settings: &Settings) {
        self.camera_controller.set_speed(settings.camera.speed);
        self.camera_controller
            .set_sensitivity(settings.camera.sensitivity);
        self.projection.set_fov(settings.camera.fov);
        let orbit_speed = if settings.camera.auto_orbit {
            settings.camera.orbit_speed
        } else {
            0.0
        };
        self.camera_controller.set_auto_orbit_speed(orbit_speed);
        if let CameraMode::Orbit { ref mut target, .. } = self.camera.mode {
            *target = settings.camera.orbit_centre.into();
        }
    }

    pub fn camera_info(&self) -> CameraInfo {
        CameraInfo {
            is_orbit: matches!(self.camera.mode, CameraMode::Orbit { .. }),
        }
    }

    pub fn handle_mouse_motion(&mut self, dx: f64, dy: f64) {
        self.camera_controller.process_mouse(dx, dy);
    }

    pub fn handle_mouse_scroll(&mut self, delta: &MouseScrollDelta) {
        self.camera_controller.process_mouse_scroll(delta);
    }

    pub fn handle_pinch(&mut self, delta: f64) {
        self.camera_controller.process_pinch(delta as f32);
    }

    pub fn handle_keyboard(&mut self, key: KeyCode, pressed: bool) {
        self.camera_controller.process_keyboard(key, pressed);
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.projection.resize(width, height);
    }

    pub fn update(&mut self, dt: Duration, settings: &mut Settings) {
        let fov_delta = self.camera_controller.update_camera(&mut self.camera, dt);
        if fov_delta != 0.0 {
            settings.camera.fov = (settings.camera.fov + fov_delta).clamp(10.0, 120.0);
            self.projection.set_fov(settings.camera.fov);
        }
    }

    pub fn view_proj(&self) -> Mat4 {
        self.projection.calc_matrix() * self.camera.calc_matrix()
    }

    pub fn scene_controller(&mut self) -> &mut SceneController {
        &mut self.scene_controller
    }

    pub fn scene_stats(&self) -> SceneStats {
        self.scene_controller.scene_stats()
    }
}
