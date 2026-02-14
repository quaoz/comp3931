use std::{f32::consts::FRAC_PI_2, time::Duration};

use glam::{Mat4, Vec3};
use winit::{dpi::PhysicalPosition, event::MouseScrollDelta, keyboard::KeyCode};

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

#[derive(Debug, Clone)]
pub enum CameraMode {
    Fps,
    Orbit {
        target: Vec3,
        azimuth: f32,
        elevation: f32,
        distance: f32,
    },
}

#[derive(Debug)]
pub struct Camera {
    pub position: Vec3,
    yaw: f32,
    pitch: f32,
    pub mode: CameraMode,
}

impl Camera {
    pub fn new<V: Into<Vec3>>(position: V, yaw: f32, pitch: f32) -> Self {
        Self {
            position: position.into(),
            yaw,
            pitch,
            mode: CameraMode::Fps,
        }
    }

    pub fn calc_matrix(&self) -> Mat4 {
        match &self.mode {
            CameraMode::Fps => {
                let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
                let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
                Mat4::look_to_rh(
                    self.position,
                    Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize(),
                    Vec3::Y,
                )
            }
            CameraMode::Orbit {
                target,
                azimuth,
                elevation,
                distance,
            } => {
                let eye = orbit_eye(*target, *azimuth, *elevation, *distance);
                Mat4::look_at_rh(eye, *target, Vec3::Y)
            }
        }
    }

    pub fn camera_position(&self) -> Vec3 {
        match &self.mode {
            CameraMode::Fps => self.position,
            CameraMode::Orbit {
                target,
                azimuth,
                elevation,
                distance,
            } => orbit_eye(*target, *azimuth, *elevation, *distance),
        }
    }

    pub fn set_orbit(&mut self, target: Vec3, distance: f32) {
        self.mode = CameraMode::Orbit {
            target,
            azimuth: std::f32::consts::FRAC_PI_4,
            elevation: 0.4,
            distance,
        };
    }

    pub fn set_fps(&mut self) {
        if let CameraMode::Orbit {
            target, distance, ..
        } = &self.mode
        {
            // Restore FPS position roughly where orbit was looking from
            self.position = orbit_eye(*target, std::f32::consts::FRAC_PI_4, 0.4, *distance);
        }
        self.mode = CameraMode::Fps;
    }
}

fn orbit_eye(target: Vec3, azimuth: f32, elevation: f32, distance: f32) -> Vec3 {
    let (el_sin, el_cos) = elevation.sin_cos();
    let (az_sin, az_cos) = azimuth.sin_cos();
    target
        + Vec3::new(
            distance * el_cos * az_cos,
            distance * el_sin,
            distance * el_cos * az_sin,
        )
}

#[derive(Debug)]
pub struct Projection {
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new<F: Into<f32>>(width: u32, height: u32, fovy: F, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn set_fov(&mut self, fov_degrees: f32) {
        self.fovy = fov_degrees.to_radians();
    }

    pub fn calc_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

#[derive(Debug)]
pub struct CameraController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    scroll: f32,
    speed: f32,
    sensitivity: f32,
    sprint: bool,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            scroll: 0.0,
            sprint: false,
            speed,
            sensitivity,
        }
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    pub fn set_sensitivity(&mut self, sensitivity: f32) {
        self.sensitivity = sensitivity;
    }

    pub fn process_keyboard(&mut self, key: KeyCode, pressed: bool) -> bool {
        let amount = if pressed { 1.0 } else { 0.0 };
        match key {
            KeyCode::KeyW => {
                self.amount_forward = amount;
                true
            }
            KeyCode::KeyS => {
                self.amount_backward = amount;
                true
            }
            KeyCode::KeyA => {
                self.amount_left = amount;
                true
            }
            KeyCode::KeyD => {
                self.amount_right = amount;
                true
            }
            KeyCode::Space => {
                self.amount_up = amount;
                true
            }
            KeyCode::ShiftLeft => {
                self.amount_down = amount;
                true
            }
            KeyCode::ControlLeft => {
                self.sprint = pressed;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal += mouse_dx as f32;
        self.rotate_vertical += mouse_dy as f32;
    }

    pub fn process_mouse_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll += match delta {
            MouseScrollDelta::LineDelta(_, scroll) => -scroll * 0.5,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => -*scroll as f32,
        };
    }

    /// Handle a trackpad pinch gesture. Positive delta = fingers spreading (zoom in).
    pub fn process_pinch(&mut self, delta: f32) {
        self.scroll -= delta * 20.0;
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        match &mut camera.mode {
            CameraMode::Fps => {
                let dt = dt.as_secs_f32();
                let speed = if self.sprint {
                    self.speed * 2.0
                } else {
                    self.speed
                };

                let (yaw_sin, yaw_cos) = camera.yaw.sin_cos();
                let forward = Vec3::new(yaw_cos, 0.0, yaw_sin).normalize();
                let right = Vec3::new(-yaw_sin, 0.0, yaw_cos).normalize();
                camera.position +=
                    forward * (self.amount_forward - self.amount_backward) * speed * dt;
                camera.position += right * (self.amount_right - self.amount_left) * speed * dt;

                let (pitch_sin, pitch_cos) = camera.pitch.sin_cos();
                let scrollward =
                    Vec3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
                camera.position += scrollward * self.scroll * speed;
                self.scroll = 0.0;

                camera.position.y += (self.amount_up - self.amount_down) * speed * dt;

                camera.yaw += self.rotate_horizontal * self.sensitivity;
                camera.pitch += -self.rotate_vertical * self.sensitivity;
                self.rotate_horizontal = 0.0;
                self.rotate_vertical = 0.0;

                camera.pitch = camera.pitch.clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);
            }
            CameraMode::Orbit {
                azimuth,
                elevation,
                distance,
                ..
            } => {
                *azimuth += self.rotate_horizontal * self.sensitivity * 3.0;
                *elevation = (*elevation - self.rotate_vertical * self.sensitivity * 3.0)
                    .clamp(0.05, SAFE_FRAC_PI_2);
                *distance = (*distance + self.scroll * self.speed * 0.3).max(1.0);
                self.rotate_horizontal = 0.0;
                self.rotate_vertical = 0.0;
                self.scroll = 0.0;
            }
        }
    }
}
