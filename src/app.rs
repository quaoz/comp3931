use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoopProxy},
    keyboard::PhysicalKey,
    window::Window,
};

use crate::state::State;

pub enum AppEvent {
    Start(State),
}

pub struct App {
    proxy: EventLoopProxy<AppEvent>,
    state: Option<State>,
}

impl App {
    pub fn new(proxy: EventLoopProxy<AppEvent>) -> Self {
        Self { proxy, state: None }
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attrs = Window::default_attributes();
        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("Failed to create window"),
        );

        let proxy = self.proxy.clone();
        std::thread::spawn(move || {
            pollster::block_on(async move {
                let state = State::new(window).await?;
                proxy
                    .send_event(AppEvent::Start(state))
                    .ok()
                    .expect("Failed to send start event");
                anyhow::Ok(())
            })
        });
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
            }
            WindowEvent::Resized(new_size) => {
                state.resize(new_size.width, new_size.height);
            }
            WindowEvent::Focused(focused) => {
                state.set_cursor_captured(*focused);
            }
            WindowEvent::RedrawRequested => {
                state.render();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                state.handle_mouse_scroll(delta);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    state.handle_key(code, event.state.is_pressed());
                }
            }
            _ => {}
        }
    }
}
