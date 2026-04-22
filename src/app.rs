use std::{sync::Arc, time::Instant};

use pollster::FutureExt;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoopProxy},
    keyboard::PhysicalKey,
    window::Window,
};

use crate::state::{Focus, State};

pub enum AppEvent {
    Start(State),
}

pub struct App {
    proxy: EventLoopProxy<AppEvent>,
    state: Option<State>,
    last_render_time: Instant,
}

impl App {
    pub fn new(proxy: EventLoopProxy<AppEvent>) -> Self {
        Self {
            proxy,
            state: None,
            last_render_time: Instant::now(),
        }
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let window_attrs = Window::default_attributes();
        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("Failed to create window"),
        );

        let state = State::new(window)
            .block_on()
            .expect("Failed to create state");

        self.proxy
            .send_event(AppEvent::Start(state))
            .ok()
            .expect("Failed to send start event");
    }

    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::Start(state) => {
                state.display.window().request_redraw();
                self.state = Some(state);
            }
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let state = match &mut self.state {
            Some(s) => s,
            None => return,
        };

        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
            state.handle_mouse_motion(dx, dy);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(s) => s,
            None => return,
        };

        match &event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            WindowEvent::Resized(new_size) => {
                state.resize(new_size.width, new_size.height);
            }
            WindowEvent::Focused(focused) => {
                if !*focused {
                    state.handle_focus(Focus::Ui);
                }
            }
            WindowEvent::RedrawRequested => {
                let dt = self.last_render_time.elapsed();
                self.last_render_time = Instant::now();
                state.render(dt);
            }
            _ => {}
        }

        // Forward event if not consumed by ui
        if !state.handle_input(&event) {
            match &event {
                WindowEvent::MouseWheel { delta, .. } => {
                    state.handle_mouse_scroll(delta);
                }
                WindowEvent::KeyboardInput {
                    event: key_event, ..
                } => {
                    if let PhysicalKey::Code(code) = key_event.physical_key {
                        state.handle_key(code, key_event.state.is_pressed());
                    }
                }
                WindowEvent::MouseInput {
                    state: winit::event::ElementState::Pressed,
                    button: winit::event::MouseButton::Left,
                    ..
                } => {
                    // Regain focus when click outside of ui
                    if let Focus::Ui = state.focus {
                        state.handle_focus(Focus::World);
                    }
                }
                WindowEvent::PinchGesture { delta, .. } => {
                    state.handle_pinch(*delta);
                }
                _ => {}
            }
        }
    }
}
