use std::sync::Arc;

use wgpu::{Device, PresentMode, Queue, Surface, SurfaceConfiguration};
use winit::window::Window;

#[derive(Debug)]
pub struct Display {
    surface: Surface<'static>,
    is_surface_configured: bool,
    pub window: Arc<Window>,
    pub config: SurfaceConfiguration,
    pub device: Device,
    pub queue: Queue,
}

impl Display {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Display> {
        // create wgpu instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all().with_env(),
            ..Default::default()
        });

        // create surface from window
        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface");

        // create logical device and command queue
        let adapter = wgpu::util::initialize_adapter_from_env_or_default(&instance, Some(&surface))
            .await
            .expect("Failed to find suitable adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("Failed to create device");

        // configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let size = window.inner_size();
        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        Ok(Self {
            is_surface_configured: false,
            surface,
            window,
            config,
            device,
            queue,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width != 0 && height != 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }

    pub fn configure(&mut self) {
        let size = self.window.inner_size();
        self.resize(size.width, size.height);
    }

    pub fn surface(&self) -> &wgpu::Surface<'static> {
        &self.surface
    }

    pub fn is_surface_configured(&self) -> bool {
        self.is_surface_configured
    }
}
