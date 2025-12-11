use glam::Vec3;

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Travel(f32),
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
    }

    pub fn do_actions<A: Into<Action> + Copy>(&mut self, actions: &[A]) {
        for action in actions {
            self.do_action(*action);
        }
    }

    pub fn do_action<A: Into<Action> + Copy>(&mut self, action: A) {
        match action.into() {
            Action::Travel(distance) => self.travel(distance),
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
}
