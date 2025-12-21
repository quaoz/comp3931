use glam::Mat4;
use wgpu::{
    BindGroup, Buffer, BufferDescriptor, BufferUsages, RenderPipeline, TextureView,
    VertexBufferLayout,
    util::{BufferInitDescriptor, DeviceExt},
};

use crate::{
    graphics::display::Display,
    world::{World, scenes::SceneBuffers},
};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

const MAX_VERTICES: u64 = 20_000_000;
const VERTEX_SIZE: u64 = 4 * 3;
const COLOR_SIZE: u64 = 4 * 3;
const NORMAL_SIZE: u64 = 4 * 3;

const VERTEX_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
    array_stride: VERTEX_SIZE,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
};

const COLOR_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
    array_stride: COLOR_SIZE,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &wgpu::vertex_attr_array![1 => Float32x3],
};

const NORMAL_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
    array_stride: NORMAL_SIZE,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &wgpu::vertex_attr_array![2 => Float32x3],
};

fn depth_stencil_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::Less,
        stencil: Default::default(),
        bias: Default::default(),
    }
}

fn create_depth_texture(device: &wgpu::Device, width: u32, height: u32) -> TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth_texture"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&Default::default())
}

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct Uniforms {
    view_proj: Mat4,
}

#[derive(Debug)]
pub struct Renderer {
    uniforms: Uniforms,
    uniform_buffer: Buffer,
    uniforms_bind_group: BindGroup,

    depth_view: TextureView,

    // Line rendering
    vertex_buffer: Buffer,
    color_buffer: Buffer,
    index_buffer: Buffer,
    line_segments: Vec<(u32, u32)>,
    draw_lines: RenderPipeline,

    // Mesh rendering
    mesh_vertex_buffer: Buffer,
    mesh_normal_buffer: Buffer,
    mesh_color_buffer: Buffer,
    mesh_index_buffer: Buffer,
    mesh_index_count: u32,
    draw_mesh: RenderPipeline,
}

impl Renderer {
    pub fn init(display: &Display) -> anyhow::Result<Self> {
        let line_shader = display
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/lines.wgsl"));

        let mesh_shader = display
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/mesh.wgsl"));

        let uniforms = Uniforms {
            view_proj: Mat4::IDENTITY,
        };
        let uniform_buffer = display.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("uniform_buffer"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
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

        let depth_view = create_depth_texture(
            &display.device,
            display.surface_config.width,
            display.surface_config.height,
        );

        // --- Line pipeline ---

        let vertex_buffer = display.device.create_buffer(&BufferDescriptor {
            label: Some("Line Vertex Buffer"),
            size: VERTEX_SIZE * MAX_VERTICES,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let color_buffer = display.device.create_buffer(&BufferDescriptor {
            label: Some("Line Color Buffer"),
            size: COLOR_SIZE * MAX_VERTICES,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = display.device.create_buffer(&BufferDescriptor {
            label: Some("Line Index Buffer"),
            size: 4 * MAX_VERTICES,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let pipeline_layout =
            display
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&uniforms_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let color_target = wgpu::ColorTargetState {
            format: display.surface_config.format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        };

        let draw_lines = display
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("draw_lines"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &line_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[VERTEX_LAYOUT, COLOR_LAYOUT],
                    compilation_options: Default::default(),
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineStrip,
                    ..Default::default()
                },
                depth_stencil: Some(depth_stencil_state()),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &line_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(color_target.clone())],
                    compilation_options: Default::default(),
                }),
                multiview: None,
                cache: None,
            });

        // --- Mesh pipeline ---

        let mesh_vertex_buffer = display.device.create_buffer(&BufferDescriptor {
            label: Some("Mesh Vertex Buffer"),
            size: VERTEX_SIZE * MAX_VERTICES,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mesh_color_buffer = display.device.create_buffer(&BufferDescriptor {
            label: Some("Mesh Color Buffer"),
            size: COLOR_SIZE * MAX_VERTICES,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mesh_normal_buffer = display.device.create_buffer(&BufferDescriptor {
            label: Some("Mesh Normal Buffer"),
            size: NORMAL_SIZE * MAX_VERTICES,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mesh_index_buffer = display.device.create_buffer(&BufferDescriptor {
            label: Some("Mesh Index Buffer"),
            size: 4 * MAX_VERTICES,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let draw_mesh = display
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("draw_mesh"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &mesh_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[VERTEX_LAYOUT, COLOR_LAYOUT, NORMAL_LAYOUT],
                    compilation_options: Default::default(),
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(depth_stencil_state()),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &mesh_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(color_target)],
                    compilation_options: Default::default(),
                }),
                multiview: None,
                cache: None,
            });

        Ok(Self {
            uniforms,
            uniform_buffer,
            uniforms_bind_group,

            depth_view,

            vertex_buffer,
            color_buffer,
            index_buffer,
            line_segments: Vec::new(),
            draw_lines,

            mesh_vertex_buffer,
            mesh_normal_buffer,
            mesh_color_buffer,
            mesh_index_buffer,
            mesh_index_count: 0,
            draw_mesh,
        })
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.depth_view = create_depth_texture(device, width, height);
    }

    pub fn update(&mut self, display: &Display, world: &mut World) {
        self.uniforms.view_proj = world.view_proj();
        display
            .queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&self.uniforms));

        let buffers = SceneBuffers {
            queue: &display.queue,
            line_vertex: &self.vertex_buffer,
            line_color: &self.color_buffer,
            line_index: &self.index_buffer,
            mesh_vertex: &self.mesh_vertex_buffer,
            mesh_normal: &self.mesh_normal_buffer,
            mesh_color: &self.mesh_color_buffer,
            mesh_index: &self.mesh_index_buffer,
        };
        let (line_segments, mesh_index_count) = world.scene_controller().set_scene(&buffers);
        self.line_segments = line_segments;
        self.mesh_index_count = mesh_index_count;
    }

    pub fn render(&mut self, display: &mut Display) {
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
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        // Draw lines
        draw_pass.set_pipeline(&self.draw_lines);
        draw_pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
        draw_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        draw_pass.set_vertex_buffer(1, self.color_buffer.slice(..));
        draw_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        for (start_index, count) in &self.line_segments {
            draw_pass.draw_indexed(*start_index..(*start_index + *count), 0, 0..1);
        }

        // Draw meshes
        if self.mesh_index_count > 0 {
            draw_pass.set_pipeline(&self.draw_mesh);
            draw_pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
            draw_pass.set_vertex_buffer(0, self.mesh_vertex_buffer.slice(..));
            draw_pass.set_vertex_buffer(1, self.mesh_color_buffer.slice(..));
            draw_pass.set_vertex_buffer(2, self.mesh_normal_buffer.slice(..));
            draw_pass.set_index_buffer(self.mesh_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            draw_pass.draw_indexed(0..self.mesh_index_count, 0, 0..1);
        }

        drop(draw_pass);

        display.queue.submit([encoder.finish()]);
        frame.present();
    }
}
