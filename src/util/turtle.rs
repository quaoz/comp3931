use std::f32::consts::TAU;

use glam::Vec3;

const CYLINDER_SEGMENTS: usize = 8;

#[derive(Debug, Clone, Copy)]
struct PathEntry {
    jump: bool,
    path_idx: usize,
    colour_idx: usize,
}

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

    taper: f32,
    stack: Vec<(usize, Vec3, Vec3)>,
    path_buf: Vec<Vec3>,
    colour_buf: Vec<Vec3>,
    path_entries: Vec<PathEntry>,
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

            taper: 1.0,
            stack: Vec::new(),
            path_buf: vec![pos],
            colour_buf: vec![colour],
            path_entries: vec![PathEntry {
                jump: true,
                path_idx: 0,
                colour_idx: 0,
            }],
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
        self.taper = 1.0;
        self.stack = Vec::new();
        self.path_buf = vec![pos];
        self.colour_buf = vec![colour];
        self.path_entries = vec![PathEntry {
            jump: true,
            path_idx: 0,
            colour_idx: 0,
        }];
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
        let entry_idx = self.path_entries.len() - 1;
        self.stack.push((entry_idx, self.normal, self.heading));
    }

    /// Pops the last position, heading, normal and colour from the stack
    pub fn pop(&mut self) {
        if let Some(state) = self.stack.pop() {
            let prev = self.path_entries[state.0];
            self.path_entries.push(PathEntry {
                jump: true,
                path_idx: prev.path_idx,
                colour_idx: prev.colour_idx,
            });
            self.normal = state.1;
            self.heading = state.2;
        }
    }

    fn pos(&self) -> Vec3 {
        self.path_buf[self.path_entries.last().unwrap().path_idx]
    }

    /// Move `distance` along current heading
    pub fn travel(&mut self, distance: f32) {
        let entry = PathEntry {
            jump: false,
            path_idx: self.path_buf.len(),
            colour_idx: self.colour_buf.len() - 1,
        };

        self.path_buf
            .push(self.pos() + (distance * self.scale) * self.heading);
        self.path_entries.push(entry);
    }

    /// Generate a cylinder mesh and advance the turtle
    pub fn branch(&mut self, length: f32, diameter: f32) {
        let tapered_diameter = diameter * self.taper.powi(self.stack.len() as i32);
        let radius = (tapered_diameter * self.scale) * 0.5;

        let base_pos = self.pos();
        let tip_pos = base_pos + (length * self.scale) * self.heading;

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
        let pos = self.pos();
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

    /// Sets the taper ratio applied per branch nesting depth
    pub fn set_taper(&mut self, taper: f32) {
        self.taper = taper;
    }

    /// Extract line geometry data without writing to GPU
    pub fn line_geometry(&self) -> LineGeometry {
        let mut vertices = Vec::new();
        let mut colors = Vec::new();
        let mut indices = Vec::new();
        let mut segments = Vec::new();

        let mut vertex_count = 0u32;
        let mut segment_start = 0u32;
        let mut segment_length = 0u32;
        let mut last_path_idx = 0;

        for entry in self.path_entries.iter().skip(1) {
            if entry.jump {
                if segment_length > 0 {
                    segments.push((segment_start, segment_length));
                    segment_start = indices.len() as u32;
                    segment_length = 0;
                }
                last_path_idx = entry.path_idx;
            } else {
                if segment_length == 0 {
                    vertices.push(self.path_buf[last_path_idx]);
                    colors.push(self.colour_buf[entry.colour_idx]);
                    indices.push(vertex_count);
                    vertex_count += 1;
                    segment_length += 1;
                }

                vertices.push(self.path_buf[entry.path_idx]);
                colors.push(self.colour_buf[entry.colour_idx]);
                indices.push(vertex_count);
                vertex_count += 1;
                segment_length += 1;

                last_path_idx = entry.path_idx;
            }
        }

        if segment_length > 0 {
            segments.push((segment_start, segment_length));
        }

        LineGeometry {
            vertices,
            colors,
            indices,
            segments,
        }
    }

    /// Extract mesh geometry data without writing to GPU
    pub fn mesh_geometry(&self) -> MeshGeometry {
        MeshGeometry {
            vertices: self.mesh_vertices.clone(),
            normals: self.mesh_normals.clone(),
            colors: self.mesh_colors.clone(),
            indices: self.mesh_indices.clone(),
        }
    }
}

// ── Geometry data structures ──

pub struct LineGeometry {
    pub vertices: Vec<Vec3>,
    pub colors: Vec<Vec3>,
    pub indices: Vec<u32>,
    pub segments: Vec<(u32, u32)>,
}

#[derive(Clone)]
pub struct MeshGeometry {
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub colors: Vec<Vec3>,
    pub indices: Vec<u32>,
}

/// Combine multiple line geometries into one, adjusting indices
pub fn combine_line_geometries(geos: &[LineGeometry]) -> LineGeometry {
    let mut vertices = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    let mut segments = Vec::new();

    for geo in geos {
        let vertex_offset = vertices.len() as u32;
        let index_offset = indices.len() as u32;

        vertices.extend_from_slice(&geo.vertices);
        colors.extend_from_slice(&geo.colors);
        indices.extend(geo.indices.iter().map(|i| i + vertex_offset));
        segments.extend(
            geo.segments
                .iter()
                .map(|(start, count)| (start + index_offset, *count)),
        );
    }

    LineGeometry {
        vertices,
        colors,
        indices,
        segments,
    }
}

/// Combine multiple mesh geometries into one, adjusting indices
pub fn combine_mesh_geometries(geos: &[MeshGeometry]) -> MeshGeometry {
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    for geo in geos {
        let vertex_offset = vertices.len() as u32;

        vertices.extend_from_slice(&geo.vertices);
        normals.extend_from_slice(&geo.normals);
        colors.extend_from_slice(&geo.colors);
        indices.extend(geo.indices.iter().map(|i| i + vertex_offset));
    }

    MeshGeometry {
        vertices,
        normals,
        colors,
        indices,
    }
}
