use std::{sync::Arc, time::Instant};

use anyhow::{Ok, Result};
use pollster::FutureExt;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    keyboard::PhysicalKey,
    window::Window,
};

use crate::framework::{display::Display, renderer::Renderer};

pub struct App<R: Renderer> {
    ctx: Option<(Display, R)>,
    proxy: Option<EventLoopProxy<Result<(Display, R)>>>,
    last_time: Instant,
}

impl<R: Renderer + 'static> App<R> {
    pub fn new(event_loop: &EventLoop<anyhow::Result<(Display, R)>>) -> Self {
        Self {
            ctx: None,
            proxy: Some(event_loop.create_proxy()),
            last_time: Instant::now(),
        }
    }
}

impl<R: Renderer> ApplicationHandler<Result<(Display, R)>> for App<R> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        if let Some(proxy) = self.proxy.take() {
            let display_future = Display::new(window.clone());
            let result = (move || {
                let display = display_future.block_on()?;
                let renderer = R::init(&display)?;

                Ok((display, renderer))
            })();

            proxy
                .send_event(result)
                .expect("Unable to send (display, renderer)");
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: Result<(Display, R)>) {
        let (display, renderer) = event.unwrap();
        display.window.request_redraw();

        self.ctx = Some((display, renderer));
        self.last_time = Instant::now();
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let (_, renderer) = match &mut self.ctx {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            DeviceEvent::Button { button, state } => {
                renderer.handle_mouse_button(button, state.is_pressed());
            }
            DeviceEvent::MouseMotion { delta: (dx, dy) } => {
                renderer.handle_mouse_move(dx, dy);
            }
            _ => {}
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if let Some((display, renderer)) = &mut self.ctx {
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::Resized(new_size) => {
                    display.resize(new_size.width, new_size.height);
                    renderer.resize(display);
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    if let PhysicalKey::Code(code) = event.physical_key {
                        renderer.handle_keyboard(code, event.state.is_pressed());
                    }
                }
                WindowEvent::RedrawRequested => {
                    display.window.request_redraw();

                    let dt = self.last_time.elapsed();
                    self.last_time = Instant::now();

                    renderer.update(display, dt);

                    if display.is_surface_configured() {
                        renderer.render(display);
                    } else {
                        display.configure();
                        renderer.resize(display);
                    }
                }
                _ => {}
            }
        }
    }
}
