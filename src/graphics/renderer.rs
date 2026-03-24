use glam::{Mat4, Vec3};
use wgpu::{
    BindGroup, Buffer, BufferDescriptor, BufferUsages, RenderPipeline, TextureView,
    VertexBufferLayout,
    util::{BufferInitDescriptor, DeviceExt},
};

use crate::{
    graphics::display::Display,
    settings::{DisplaySettings, Settings, season_tint},
    world::{
        World,
        scenes::{FrameParams, SceneBuffers},
    },
};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const MAX_VERTICES: u64 = 20_000_000;
const VEC3_SIZE: u64 = 4 * 3;
const VEC2_SIZE: u64 = 4 * 2;

/// Vertex layout for one tightly-packed per-vertex attribute at `shader_location`.
const fn attribute_layout(
    stride: u64,
    attributes: &'static [wgpu::VertexAttribute],
) -> VertexBufferLayout<'static> {
    VertexBufferLayout {
        array_stride: stride,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes,
    }
}

const VERTEX_LAYOUT: VertexBufferLayout<'static> =
    attribute_layout(VEC3_SIZE, &wgpu::vertex_attr_array![0 => Float32x3]);
const COLOUR_LAYOUT: VertexBufferLayout<'static> =
    attribute_layout(VEC3_SIZE, &wgpu::vertex_attr_array![1 => Float32x3]);
const NORMAL_LAYOUT: VertexBufferLayout<'static> =
    attribute_layout(VEC3_SIZE, &wgpu::vertex_attr_array![2 => Float32x3]);
const UV_LAYOUT: VertexBufferLayout<'static> =
    attribute_layout(VEC2_SIZE, &wgpu::vertex_attr_array![3 => Float32x2]);
const TANGENT_LAYOUT: VertexBufferLayout<'static> =
    attribute_layout(VEC3_SIZE, &wgpu::vertex_attr_array![4 => Float32x3]);

fn load_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bytes: &[u8],
    format: wgpu::TextureFormat,
    label: &str,
) -> anyhow::Result<wgpu::TextureView> {
    let img = image::load_from_memory(bytes)?.to_rgba8();
    let (width, height) = img.dimensions();
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            aspect: wgpu::TextureAspect::All,
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        &img,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        size,
    );

    Ok(texture.create_view(&Default::default()))
}

/// Allocates a `MAX_VERTICES`-capacity buffer of `element_size`-byte elements.
fn geometry_buffer(
    device: &wgpu::Device,
    label: &str,
    element_size: u64,
    usage: BufferUsages,
) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: Some(label),
        size: element_size * MAX_VERTICES,
        usage: usage | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

/// As [`geometry_buffer`], for a `u32` index buffer.
fn alloc_index_buffer(device: &wgpu::Device, label: &str) -> Buffer {
    geometry_buffer(device, label, 4, BufferUsages::INDEX)
}

/// As [`geometry_buffer`], for a vertex attribute buffer.
fn alloc_vertex_buffer(device: &wgpu::Device, label: &str, element_size: u64) -> Buffer {
    geometry_buffer(device, label, element_size, BufferUsages::VERTEX)
}

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
    view_proj: Mat4,         // 0–63
    light_dir: Vec3,         // 64–75
    ambient: f32,            // 76–79
    season_tint: Vec3,       // 80–91
    time: f32,               // 92–95
    wind_anim_strength: f32, // 96–99
    wind_dir_x: f32,         // 100–103
    wind_dir_z: f32,         // 104–107
    _pad: f32,               // 108–111  (total 112 bytes, align 16)
}

#[derive(Debug)]
pub struct Renderer {
    uniform_buffer: Buffer,
    uniforms_bind_group: BindGroup,

    depth_view: TextureView,

    // Line rendering
    vertex_buffer: Buffer,
    colour_buffer: Buffer,
    index_buffer: Buffer,
    line_segments: Vec<(u32, u32)>,
    debug_line_segments: Vec<(u32, u32)>,
    draw_lines: RenderPipeline,

    // Mesh rendering
    mesh_vertex_buffer: Buffer,
    mesh_normal_buffer: Buffer,
    mesh_colour_buffer: Buffer,
    mesh_uv_buffer: Buffer,
    mesh_tangent_buffer: Buffer,
    mesh_index_buffer: Buffer,
    mesh_index_count: u32,
    leaf_texture_bind_group: BindGroup,
    draw_mesh: RenderPipeline,
}

impl Renderer {
    pub fn init(display: &Display) -> anyhow::Result<Self> {
        let line_shader = display
            .device
            .create_shader_module(wgpu::include_wgsl!("../../assets/shaders/lines.wgsl"));

        let mesh_shader = display
            .device
            .create_shader_module(wgpu::include_wgsl!("../../assets/shaders/mesh.wgsl"));

        let uniform_buffer = display.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("uniform_buffer"),
            contents: &[0u8; std::mem::size_of::<Uniforms>()],
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
        });

        let uniforms_bind_group_layout =
            display
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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

        // Leaf textures (3×3 atlas, 9 leaf variants)
        let colour_view = load_texture(
            &display.device,
            &display.queue,
            include_bytes!("../../assets/textures/leaves/LeafSet024_1K-PNG_Color.png"),
            wgpu::TextureFormat::Rgba8UnormSrgb,
            "leaf_colour",
        )?;
        let opacity_view = load_texture(
            &display.device,
            &display.queue,
            include_bytes!("../../assets/textures/leaves/LeafSet024_1K-PNG_Opacity.png"),
            wgpu::TextureFormat::Rgba8Unorm,
            "leaf_opacity",
        )?;
        let normal_view = load_texture(
            &display.device,
            &display.queue,
            include_bytes!("../../assets/textures/leaves/LeafSet024_1K-PNG_NormalGL.png"),
            wgpu::TextureFormat::Rgba8Unorm,
            "leaf_normal",
        )?;

        let leaf_sampler = display.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let tex_entry = |binding: u32| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            },
            count: None,
        };

        let texture_bind_group_layout =
            display
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("leaf_texture_layout"),
                    entries: &[
                        tex_entry(0),
                        tex_entry(1),
                        tex_entry(2),
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let leaf_texture_bind_group =
            display
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("leaf_texture_bind_group"),
                    layout: &texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&colour_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&opacity_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&normal_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::Sampler(&leaf_sampler),
                        },
                    ],
                });

        let (width, height) = display.size();
        let depth_view = create_depth_texture(&display.device, width, height);

        // Line pipeline
        let device = &display.device;
        let vertex_buffer = alloc_vertex_buffer(device, "Line Vertex Buffer", VEC3_SIZE);
        let colour_buffer = alloc_vertex_buffer(device, "Line Colour Buffer", VEC3_SIZE);
        let index_buffer = alloc_index_buffer(device, "Line Index Buffer");

        let pipeline_layout =
            display
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&uniforms_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let mesh_pipeline_layout =
            display
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&uniforms_bind_group_layout, &texture_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let colour_target = wgpu::ColorTargetState {
            format: display.format(),
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
                    buffers: &[VERTEX_LAYOUT, COLOUR_LAYOUT],
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
                    targets: &[Some(colour_target.clone())],
                    compilation_options: Default::default(),
                }),
                multiview: None,
                cache: None,
            });

        // Mesh pipeline
        let mesh_vertex_buffer = alloc_vertex_buffer(device, "Mesh Vertex Buffer", VEC3_SIZE);
        let mesh_colour_buffer = alloc_vertex_buffer(device, "Mesh Colour Buffer", VEC3_SIZE);
        let mesh_normal_buffer = alloc_vertex_buffer(device, "Mesh Normal Buffer", VEC3_SIZE);
        let mesh_uv_buffer = alloc_vertex_buffer(device, "Mesh UV Buffer", VEC2_SIZE);
        let mesh_tangent_buffer = alloc_vertex_buffer(device, "Mesh Tangent Buffer", VEC3_SIZE);
        let mesh_index_buffer = alloc_index_buffer(device, "Mesh Index Buffer");

        let draw_mesh = display
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("draw_mesh"),
                layout: Some(&mesh_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &mesh_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[
                        VERTEX_LAYOUT,
                        COLOUR_LAYOUT,
                        NORMAL_LAYOUT,
                        UV_LAYOUT,
                        TANGENT_LAYOUT,
                    ],
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
                    targets: &[Some(colour_target)],
                    compilation_options: Default::default(),
                }),
                multiview: None,
                cache: None,
            });

        Ok(Self {
            uniform_buffer,
            uniforms_bind_group,

            depth_view,

            vertex_buffer,
            colour_buffer,
            index_buffer,
            line_segments: Vec::new(),
            debug_line_segments: Vec::new(),
            draw_lines,

            mesh_vertex_buffer,
            mesh_normal_buffer,
            mesh_colour_buffer,
            mesh_uv_buffer,
            mesh_tangent_buffer,
            mesh_index_buffer,
            mesh_index_count: 0,
            leaf_texture_bind_group,
            draw_mesh,
        })
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.depth_view = create_depth_texture(device, width, height);
    }

    pub fn mesh_index_count(&self) -> u32 {
        self.mesh_index_count
    }

    pub fn update(
        &mut self,
        display: &Display,
        world: &mut World,
        settings: &mut Settings,
        elapsed_secs: f32,
    ) {
        // Wind direction vector from compass azimuth (0° = north/+Z, 90° = east/+X).
        let az = settings.env.wind_azimuth.to_radians();
        let uniforms = Uniforms {
            view_proj: world.view_proj(),
            light_dir: Vec3::from_array(settings.env.light_position).normalize_or(Vec3::Y),
            ambient: settings.env.ambient,
            season_tint: season_tint(settings.scene.active().date.season()),
            time: elapsed_secs,
            wind_anim_strength: settings.env.wind_anim_strength,
            wind_dir_x: az.sin(),
            wind_dir_z: az.cos(),
            _pad: 0.0,
        };
        display
            .queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        let ground_colour = Vec3::from_array(settings.display.ground_colour);
        let camera_pos = world.camera_position();
        let view_proj = world.view_proj();

        let buffers = SceneBuffers {
            queue: &display.queue,
            line_vertex: &self.vertex_buffer,
            line_colour: &self.colour_buffer,
            line_index: &self.index_buffer,
            mesh_vertex: &self.mesh_vertex_buffer,
            mesh_normal: &self.mesh_normal_buffer,
            mesh_colour: &self.mesh_colour_buffer,
            mesh_uv: &self.mesh_uv_buffer,
            mesh_tangent: &self.mesh_tangent_buffer,
            mesh_index: &self.mesh_index_buffer,
        };
        let frame = FrameParams {
            camera_pos,
            view_proj,
            buffers: &buffers,
            ground_colour,
            debug_mode: settings.display.debug_mode,
        };
        let out = world.scene_controller().set_scene(
            &mut settings.scene,
            &mut settings.env,
            &settings.lod,
            &settings.cull,
            &frame,
        );
        self.line_segments = out.line_segments;
        self.debug_line_segments = out.debug_line_segments;
        self.mesh_index_count = out.mesh_index_count;
    }

    pub fn render_scene(
        &mut self,
        display: &mut Display,
        settings: &DisplaySettings,
    ) -> Option<(wgpu::SurfaceTexture, TextureView)> {
        let frame = match display.surface().get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated) => return None,
            Err(e) => panic!("{}", e),
        };

        let view = frame.texture.create_view(&Default::default());
        self.render_to_view(display, &view, &self.depth_view, settings);

        Some((frame, view))
    }

    /// Render the current scene into an arbitrary `TextureView`
    pub fn render_to_view(
        &self,
        display: &Display,
        view: &TextureView,
        depth_view: &TextureView,
        settings: &DisplaySettings,
    ) {
        let mut encoder = display.device.create_command_encoder(&Default::default());
        {
            let mut draw_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("draw_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(settings.clear_colour()),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            if settings.show_lines || (settings.debug_mode && !self.debug_line_segments.is_empty())
            {
                draw_pass.set_pipeline(&self.draw_lines);
                draw_pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
                draw_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                draw_pass.set_vertex_buffer(1, self.colour_buffer.slice(..));
                draw_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                if settings.show_lines {
                    for (start_index, count) in &self.line_segments {
                        draw_pass.draw_indexed(*start_index..(*start_index + *count), 0, 0..1);
                    }
                }

                if settings.debug_mode {
                    for (start_index, count) in &self.debug_line_segments {
                        draw_pass.draw_indexed(*start_index..(*start_index + *count), 0, 0..1);
                    }
                }
            }

            if settings.show_meshes && self.mesh_index_count > 0 {
                draw_pass.set_pipeline(&self.draw_mesh);
                draw_pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
                draw_pass.set_bind_group(1, &self.leaf_texture_bind_group, &[]);
                draw_pass.set_vertex_buffer(0, self.mesh_vertex_buffer.slice(..));
                draw_pass.set_vertex_buffer(1, self.mesh_colour_buffer.slice(..));
                draw_pass.set_vertex_buffer(2, self.mesh_normal_buffer.slice(..));
                draw_pass.set_vertex_buffer(3, self.mesh_uv_buffer.slice(..));
                draw_pass.set_vertex_buffer(4, self.mesh_tangent_buffer.slice(..));
                draw_pass
                    .set_index_buffer(self.mesh_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                draw_pass.draw_indexed(0..self.mesh_index_count, 0, 0..1);
            }
        }
        display.queue.submit([encoder.finish()]);
    }
}
