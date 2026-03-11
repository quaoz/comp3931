use std::sync::Arc;

use wgpu::TextureFormat;
use winit::window::Window;

#[derive(Debug)]
pub struct Display {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Display {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Display> {
        // Create wgpu instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all().with_env(),
            ..Default::default()
        });

        // Create surface from window
        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface");

        // Create logical device and command queue
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

        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let size = window.inner_size();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        Ok(Self {
            surface,
            surface_config,
            is_surface_configured: false,
            window,
            adapter,
            device,
            queue,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width != 0 && height != 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
            self.is_surface_configured = true;
        }
    }

    pub fn configure(&mut self) {
        let size = self.window.inner_size();
        self.resize(size.width, size.height);
    }

    pub fn size(&self) -> (u32, u32) {
        (self.surface_config.width, self.surface_config.height)
    }

    pub fn format(&self) -> TextureFormat {
        self.surface_config.format
    }

    pub fn surface(&self) -> &wgpu::Surface<'static> {
        &self.surface
    }

    pub fn is_surface_configured(&self) -> bool {
        self.is_surface_configured
    }

    pub fn set_vsync(&mut self, vsync: bool) {
        let new_mode = if vsync {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        };
        if self.surface_config.present_mode == new_mode {
            return;
        }
        self.surface_config.present_mode = new_mode;
        if self.is_surface_configured {
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    /// Returns `true` when the surface format is BGRA (channels must be swapped for readback).
    pub fn is_bgra(&self) -> bool {
        matches!(
            self.surface_config.format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        )
    }

    /// Add or remove `COPY_SRC` from the surface texture usage.
    /// Required before reading back surface pixels for timelapse capture.
    /// Reconfigures the surface immediately if it is already configured.
    pub fn set_copy_src(&mut self, enabled: bool) {
        let new_usage = if enabled {
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC
        } else {
            wgpu::TextureUsages::RENDER_ATTACHMENT
        };

        if self.surface_config.usage != new_usage {
            self.surface_config.usage = new_usage;
            if self.is_surface_configured {
                self.surface.configure(&self.device, &self.surface_config);
            }
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }
}
