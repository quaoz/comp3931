use std::{f32::consts::PI, time::Duration};

use glam::{Mat4, vec3};
use wgpu::{
    BindGroup, Buffer, RenderPipeline,
    util::{BufferInitDescriptor, DeviceExt},
};
use winit::keyboard::KeyCode;

use crate::{
    framework::{
        camera::{Camera, CameraController, Projection},
        display::Display,
        renderer::Renderer,
    },
    turtle::scenes::SceneController,
};

const MAX_LINES: u64 = 1000000;
const LINE_SIZE: u64 = 4 * 3 * 3;
const LINE_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: LINE_SIZE,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x3,
        2 => Float32x3,
    ],
};

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct Uniforms {
    view_proj: Mat4,
}

#[derive(Debug)]
pub struct TurtleRenderer {
    camera: Camera,
    projection: Projection,
    camera_controller: CameraController,

    uniforms: Uniforms,
    uniform_buffer: Buffer,
    uniforms_bind_group: BindGroup,
    uniforms_dirty: bool,

    lines_buffer: Buffer,
    lines_count: u32,
    draw_lines: RenderPipeline,

    scene_controller: SceneController,
}

impl Renderer for TurtleRenderer {
    fn init(display: &Display) -> anyhow::Result<Self> {
        let shader = display
            .device
            .create_shader_module(wgpu::include_wgsl!("lines.wgsl"));

        let camera = Camera::new(vec3(0.0, 0.0, -50.0), std::f32::consts::FRAC_PI_2, 0.0);
        let camera_controller = CameraController::new(10.0, 0.001);
        let projection = Projection::new(
            display.config.width,
            display.config.height,
            PI * 0.25,
            0.1,
            1000.0,
        );

        let uniforms = Uniforms {
            view_proj: projection.calc_matrix() * camera.calc_matrix(),
        };
        let uniform_buffer = display.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("uniform_buffer"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });

        let uniforms_bind_group_layout =
            display
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let uniforms_bind_group = display
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("uniforms_bind_group"),
                layout: &uniforms_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
            });

        let lines_buffer = display.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Line Buffer"),
            size: LINE_SIZE * MAX_LINES,
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let draw_lines_layout =
            display
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&uniforms_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let draw_lines = display
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("draw_lines"),
                layout: Some(&draw_lines_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[LINE_LAYOUT],
                    compilation_options: Default::default(),
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: display.config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                multiview: None,
                cache: None,
            });

        let scene_controller = SceneController::new();

        Ok(Self {
            camera,
            projection,
            camera_controller,

            uniforms,
            uniform_buffer,
            uniforms_bind_group,
            uniforms_dirty: true,

            lines_buffer,
            lines_count: 0,
            draw_lines,

            scene_controller,
        })
    }

    fn handle_mouse_move(&mut self, dx: f64, dy: f64) {
        self.camera_controller.process_mouse(dx, dy);
        self.uniforms_dirty = true;
    }

    fn handle_keyboard(&mut self, key: KeyCode, pressed: bool) {
        self.camera_controller.process_keyboard(key, pressed);
        self.uniforms_dirty = true;
        if self.scene_controller.handle_keyboard(key, pressed) {};
    }

    fn resize(&mut self, display: &Display) {
        self.projection
            .resize(display.config.width, display.config.height);
        self.uniforms_dirty = true;
    }

    fn update(&mut self, display: &Display, dt: Duration) {
        // update uniforms
        if self.uniforms_dirty {
            self.uniforms_dirty = false;
            self.camera_controller.update_camera(&mut self.camera, dt);
            let proj = self.projection.calc_matrix();
            let view = self.camera.calc_matrix();

            self.uniforms.view_proj = proj * view;
            display
                .queue
                .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&self.uniforms));
        }

        self.lines_count = self
            .scene_controller
            .set_scene(&display.queue, &self.lines_buffer);
    }

    fn render(&mut self, display: &mut Display) {
        let frame = match display.surface().get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated) => return,
            Err(e) => panic!("{}", e),
        };

        let view = frame.texture.create_view(&Default::default());

        let mut encoder = display.device.create_command_encoder(&Default::default());

        let mut draw_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("draw_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        draw_pass.set_pipeline(&self.draw_lines);
        draw_pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
        draw_pass.set_vertex_buffer(0, self.lines_buffer.slice(..));
        draw_pass.draw(0..2, 0..self.lines_count);

        drop(draw_pass);

        display.queue.submit([encoder.finish()]);
        frame.present();
    }
}
