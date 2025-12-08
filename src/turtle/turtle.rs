use glam::Vec3;

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Travel(f32),
    Roll(f32),
    Turn(f32),
    Colour(Vec3),
    Push,
    Pop,
}

#[derive(Debug)]
pub struct Turtle {
    heading: Vec3,
    normal: Vec3,
    stack: Vec<(usize, Vec3, Vec3)>,
    pub path_buf: Vec<Vec3>,
    pub colour_buf: Vec<Vec3>,
    pub path_indicies: Vec<(bool, usize, usize)>,
}

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
#[repr(C)]
pub struct Line {
    pub start: Vec3,
    pub end: Vec3,
    pub colour: Vec3,
}

impl Turtle {
    pub fn new(pos: Vec3, colour: Vec3) -> Self {
        Turtle {
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
            self.heading = state.1;
            self.normal = state.2;
        }
    }

    /// Move `distance` along current heading
    pub fn travel(&mut self, distance: f32) {
        let pos = self.path_buf[self.path_indicies.last().unwrap().1];
        let idx = (false, self.path_buf.len(), self.colour_buf.len() - 1);

        self.path_buf.push(pos + distance * self.heading);
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
}
