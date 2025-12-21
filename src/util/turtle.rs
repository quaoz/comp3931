use std::f32::consts::TAU;

use glam::Vec3;

const CYLINDER_SEGMENTS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Travel(f32),
    Branch(f32, f32),
    Leaf(f32, f32),
    Roll(f32),
    Turn(f32),
    Colour(Vec3),
    Push,
    Pop,
    Nop,
}

#[derive(Debug)]
pub struct Turtle {
    scale: f32,
    heading: Vec3,
    normal: Vec3,
    stack: Vec<(usize, Vec3, Vec3)>,
    path_buf: Vec<Vec3>,
    colour_buf: Vec<Vec3>,
    path_indicies: Vec<(bool, usize, usize)>,
    mesh_vertices: Vec<Vec3>,
    mesh_normals: Vec<Vec3>,
    mesh_colors: Vec<Vec3>,
    mesh_indices: Vec<u32>,
}

impl Turtle {
    pub fn new(pos: Vec3, colour: Vec3) -> Self {
        Turtle {
            scale: 1.0,
            heading: Vec3::X,
            normal: Vec3::Y,
            stack: Vec::new(),
            path_buf: vec![pos],
            colour_buf: vec![colour],
            path_indicies: vec![(true, 0, 0)],
            mesh_vertices: Vec::new(),
            mesh_normals: Vec::new(),
            mesh_colors: Vec::new(),
            mesh_indices: Vec::new(),
        }
    }

    /// Resets the turtles path, position, heading, normal and colour
    pub fn reset(&mut self, pos: Vec3, colour: Vec3) {
        self.scale = 1.0;
        self.heading = Vec3::X;
        self.normal = Vec3::Y;
        self.stack = Vec::new();
        self.path_buf = vec![pos];
        self.colour_buf = vec![colour];
        self.path_indicies = vec![(true, 0, 0)];
        self.mesh_vertices.clear();
        self.mesh_normals.clear();
        self.mesh_colors.clear();
        self.mesh_indices.clear();
    }

    pub fn do_actions<A: Into<Action> + Copy>(&mut self, actions: &[A]) {
        for action in actions {
            self.do_action(*action);
        }
    }

    pub fn do_action<A: Into<Action> + Copy>(&mut self, action: A) {
        match action.into() {
            Action::Travel(distance) => self.travel(distance),
            Action::Branch(length, diameter) => self.branch(length, diameter),
            Action::Leaf(width, height) => self.leaf(width, height),
            Action::Roll(angle) => self.roll(angle),
            Action::Turn(angle) => self.turn(angle),
            Action::Colour(colour) => self.set_colour(colour),
            Action::Push => self.push(),
            Action::Pop => self.pop(),
            Action::Nop => (),
        };
    }

    /// Pushes the current position, heading, normal and colour to the stack
    pub fn push(&mut self) {
        let path_idx = self.path_indicies.len();
        self.stack.push((path_idx - 1, self.normal, self.heading));
    }

    /// Pops the last position, heading, normal and colour from the stack
    pub fn pop(&mut self) {
        if let Some(state) = self.stack.pop() {
            let idx = (
                true,
                self.path_indicies[state.0].1,
                self.path_indicies[state.0].2,
            );
            self.path_indicies.push(idx);
            self.normal = state.1;
            self.heading = state.2;
        }
    }

    /// Move `distance` along current heading
    pub fn travel(&mut self, distance: f32) {
        let pos = self.path_buf[self.path_indicies.last().unwrap().1];
        let idx = (false, self.path_buf.len(), self.colour_buf.len() - 1);

        self.path_buf
            .push(pos + (distance * self.scale) * self.heading);
        self.path_indicies.push(idx);
    }

    /// Generate a cylinder mesh and advance the turtle
    pub fn branch(&mut self, length: f32, diameter: f32) {
        let base_pos = self.path_buf[self.path_indicies.last().unwrap().1];
        let tip_pos = base_pos + (length * self.scale) * self.heading;
        let radius = (diameter * self.scale) * 0.5;

        let binormal = self.heading.cross(self.normal).normalize();
        let colour = *self.colour_buf.last().unwrap();
        let base_idx = self.mesh_vertices.len() as u32;

        for i in 0..CYLINDER_SEGMENTS {
            let angle = (i as f32 / CYLINDER_SEGMENTS as f32) * TAU;
            let (sin, cos) = angle.sin_cos();
            let radial = cos * self.normal + sin * binormal;
            let offset = radial * radius;

            // Base vertex
            self.mesh_vertices.push(base_pos + offset);
            self.mesh_normals.push(radial);
            self.mesh_colors.push(colour);

            // Tip vertex
            self.mesh_vertices.push(tip_pos + offset);
            self.mesh_normals.push(radial);
            self.mesh_colors.push(colour);
        }

        // Generate quad indices for each segment (2 triangles per quad)
        for i in 0..CYLINDER_SEGMENTS as u32 {
            let next = (i + 1) % CYLINDER_SEGMENTS as u32;
            let b0 = base_idx + i * 2;
            let t0 = base_idx + i * 2 + 1;
            let b1 = base_idx + next * 2;
            let t1 = base_idx + next * 2 + 1;

            // Triangle 1
            self.mesh_indices.push(b0);
            self.mesh_indices.push(t0);
            self.mesh_indices.push(b1);

            // Triangle 2
            self.mesh_indices.push(b1);
            self.mesh_indices.push(t0);
            self.mesh_indices.push(t1);
        }

        // Advance the turtle
        self.travel(length);
    }

    /// Generate a double-sided leaf quad at the current position
    pub fn leaf(&mut self, width: f32, height: f32) {
        let pos = self.path_buf[self.path_indicies.last().unwrap().1];
        let binormal = self.heading.cross(self.normal).normalize();
        let colour = *self.colour_buf.last().unwrap();

        let half_w = (width * self.scale) * 0.5;
        let h = height * self.scale;

        let base_idx = self.mesh_vertices.len() as u32;

        // 4 vertices: bottom-left, bottom-right, top-left, top-right
        let bl = pos - binormal * half_w;
        let br = pos + binormal * half_w;
        let tl = pos - binormal * half_w + self.heading * h;
        let tr = pos + binormal * half_w + self.heading * h;

        // Front face vertices (normal pointing outward)
        self.mesh_vertices.push(bl);
        self.mesh_vertices.push(br);
        self.mesh_vertices.push(tl);
        self.mesh_vertices.push(tr);
        for _ in 0..4 {
            self.mesh_normals.push(self.normal);
            self.mesh_colors.push(colour);
        }

        // Back face vertices (normal pointing opposite)
        self.mesh_vertices.push(bl);
        self.mesh_vertices.push(br);
        self.mesh_vertices.push(tl);
        self.mesh_vertices.push(tr);
        for _ in 0..4 {
            self.mesh_normals.push(-self.normal);
            self.mesh_colors.push(colour);
        }

        // Front face triangles
        self.mesh_indices.push(base_idx);
        self.mesh_indices.push(base_idx + 2);
        self.mesh_indices.push(base_idx + 1);
        self.mesh_indices.push(base_idx + 1);
        self.mesh_indices.push(base_idx + 2);
        self.mesh_indices.push(base_idx + 3);

        // Back face triangles (reversed winding)
        self.mesh_indices.push(base_idx + 4);
        self.mesh_indices.push(base_idx + 5);
        self.mesh_indices.push(base_idx + 6);
        self.mesh_indices.push(base_idx + 5);
        self.mesh_indices.push(base_idx + 7);
        self.mesh_indices.push(base_idx + 6);
    }

    /// Turn clockwise by `angle` around current normal
    pub fn turn(&mut self, angle: f32) {
        self.heading = self.heading.rotate_axis(self.normal, angle).normalize();
    }

    /// Roll clockwise by `angle` around current heading
    pub fn roll(&mut self, angle: f32) {
        self.normal = self.normal.rotate_axis(self.heading, angle).normalize();
    }

    /// Set the line colour
    pub fn set_colour(&mut self, colour: Vec3) {
        self.colour_buf.push(colour);
    }

    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }

    /// Write the turtle's path to GPU buffers using line strips
    /// Returns a Vec of (start_index, count) pairs for each continuous line segment
    pub fn write_to_buffers(
        &self,
        queue: &wgpu::Queue,
        vertex_buffer: &wgpu::Buffer,
        color_buffer: &wgpu::Buffer,
        index_buffer: &wgpu::Buffer,
    ) -> Vec<(u32, u32)> {
        let mut vertices = Vec::new();
        let mut colors = Vec::new();
        let mut indices = Vec::new();
        let mut segments = Vec::new(); // (start_index, count) for each line strip

        let mut vertex_count = 0u32;
        let mut segment_start = 0u32;
        let mut segment_length = 0u32;
        let mut last_path_idx = 0;

        for (jump, path_idx, colour_idx) in self.path_indicies.iter().skip(1) {
            if *jump {
                // End the current segment if it has any vertices
                if segment_length > 0 {
                    segments.push((segment_start, segment_length));
                    segment_start = indices.len() as u32;
                    segment_length = 0;
                }
                last_path_idx = *path_idx;
            } else {
                // For the first vertex of a new segment, add the starting point
                if segment_length == 0 {
                    vertices.push(self.path_buf[last_path_idx]);
                    colors.push(self.colour_buf[*colour_idx]);
                    indices.push(vertex_count);
                    vertex_count += 1;
                    segment_length += 1;
                }

                // Add the end point of this line
                vertices.push(self.path_buf[*path_idx]);
                colors.push(self.colour_buf[*colour_idx]);
                indices.push(vertex_count);
                vertex_count += 1;
                segment_length += 1;

                last_path_idx = *path_idx;
            }
        }

        // Don't forget the last segment
        if segment_length > 0 {
            segments.push((segment_start, segment_length));
        }

        // Write to GPU buffers
        queue.write_buffer(vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        queue.write_buffer(color_buffer, 0, bytemuck::cast_slice(&colors));
        queue.write_buffer(index_buffer, 0, bytemuck::cast_slice(&indices));

        segments
    }

    /// Write the turtle's mesh data to GPU buffers
    /// Returns the total number of indices
    pub fn write_mesh_to_buffers(
        &self,
        queue: &wgpu::Queue,
        vertex_buffer: &wgpu::Buffer,
        normal_buffer: &wgpu::Buffer,
        color_buffer: &wgpu::Buffer,
        index_buffer: &wgpu::Buffer,
    ) -> u32 {
        if self.mesh_indices.is_empty() {
            return 0;
        }

        queue.write_buffer(vertex_buffer, 0, bytemuck::cast_slice(&self.mesh_vertices));
        queue.write_buffer(normal_buffer, 0, bytemuck::cast_slice(&self.mesh_normals));
        queue.write_buffer(color_buffer, 0, bytemuck::cast_slice(&self.mesh_colors));
        queue.write_buffer(index_buffer, 0, bytemuck::cast_slice(&self.mesh_indices));

        self.mesh_indices.len() as u32
    }
}
