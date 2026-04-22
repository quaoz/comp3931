use std::{collections::HashSet, f32::consts::TAU};

use glam::Vec3;

use crate::util::{fnv::FnvBuildHasher, rng};

type VoxelSet = HashSet<(i32, i32, i32), FnvBuildHasher>;

const DEFAULT_CYLINDER_SEGMENTS: usize = 8;

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
    Pitch(f32),
    Colour(Vec3),
    Push,
    Pop,
    Cut,
    Nop,
}

/// Per-branch state saved on Push and restored on Pop
#[derive(Debug, Clone, Copy)]
struct StackEntry {
    entry_idx: usize,
    normal: Vec3,
    heading: Vec3,
    prop_curvature: f32,
    prop_bend_axis: Vec3,
    ou_drift: Vec3,
    branch_arc_length: f32,
    colour_drift: Vec3,
}

#[derive(Debug)]
pub struct Turtle {
    scale: f32,
    heading: Vec3,
    normal: Vec3,
    // Tropism
    tropism_direction: Vec3,
    tropism_strength: f32,
    gravitropism_strength: f32,
    // Wind
    wind_direction: Vec3,
    wind_strength: f32,
    wind_turbulence: f32,
    wind_baked: bool,
    // Taper
    taper: f32,
    // LOD
    lod_segments: usize,
    lod_min_radius: f32,
    // Proprioception (straightening)
    prop_curvature: f32,
    prop_bend_axis: Vec3,
    proprioception_gamma: f32,
    // Heading drift
    ou_drift: Vec3,
    variation_noise_strength: f32,
    variation_decay_rate: f32,
    // Arc-length along the current branch segment
    branch_arc_length: f32,
    // Per-branch colour drift
    colour_drift: Vec3,
    colour_variation: f32,
    // Space pruning
    occupancy_grid: VoxelSet,
    current_plant_marks: VoxelSet,
    occupancy_cell_size: f32,
    space_pruning: bool,
    // Leaf skip probability (0.0 = never skip, 1.0 = always skip)
    leaf_skip_prob: f32,
    // Internal buffers
    stack: Vec<StackEntry>,
    path_buf: Vec<Vec3>,
    colour_buf: Vec<Vec3>,
    path_entries: Vec<PathEntry>,
    mesh_vertices: Vec<Vec3>,
    mesh_normals: Vec<Vec3>,
    mesh_colours: Vec<Vec3>,
    mesh_indices: Vec<u32>,
    mesh_uvs: Vec<[f32; 2]>,
    mesh_tangents: Vec<Vec3>,
}

impl Default for Turtle {
    fn default() -> Self {
        Turtle {
            scale: 1.0,
            heading: Vec3::X,
            normal: Vec3::Y,
            tropism_direction: Vec3::Y,
            tropism_strength: 0.0,
            gravitropism_strength: 0.0,
            wind_direction: Vec3::X,
            wind_strength: 0.0,
            wind_turbulence: 0.0,
            wind_baked: true,
            taper: 1.0,
            lod_segments: DEFAULT_CYLINDER_SEGMENTS,
            lod_min_radius: 0.0,
            prop_curvature: 0.0,
            prop_bend_axis: Vec3::Y,
            proprioception_gamma: 0.0,
            ou_drift: Vec3::ZERO,
            variation_noise_strength: 0.0,
            variation_decay_rate: 0.1,
            branch_arc_length: 0.0,
            colour_drift: Vec3::ZERO,
            colour_variation: 0.0,
            occupancy_grid: HashSet::with_hasher(FnvBuildHasher::new()),
            current_plant_marks: HashSet::with_hasher(FnvBuildHasher::new()),
            occupancy_cell_size: 0.5,
            space_pruning: false,
            leaf_skip_prob: 0.0,
            stack: Vec::new(),
            path_buf: Vec::new(),
            colour_buf: Vec::new(),
            path_entries: Vec::new(),
            mesh_vertices: Vec::new(),
            mesh_normals: Vec::new(),
            mesh_colours: Vec::new(),
            mesh_indices: Vec::new(),
            mesh_uvs: Vec::new(),
            mesh_tangents: Vec::new(),
        }
    }
}

impl Turtle {
    pub fn new(pos: Vec3, colour: Vec3) -> Self {
        let mut turtle = Self::default();
        turtle.start_path(pos, colour);
        turtle
    }

    /// Seeds the path buffers with the starting position and colour
    fn start_path(&mut self, pos: Vec3, colour: Vec3) {
        self.path_buf.push(pos);
        self.colour_buf.push(colour);
        self.path_entries.push(PathEntry {
            jump: true,
            path_idx: 0,
            colour_idx: 0,
        });
    }

    /// Resets the turtle's path, position, heading, normal, colour, and all per-plant state
    pub fn reset(&mut self, pos: Vec3, colour: Vec3) {
        // Flush this plant's marks into the shared grid so that subsequent plants will be pruned
        self.occupancy_grid.extend(self.current_plant_marks.drain());

        // Reset reusing previous allocations
        self.heading = Vec3::X;
        self.normal = Vec3::Y;
        self.stack.clear();
        self.path_buf.clear();
        self.colour_buf.clear();
        self.path_entries.clear();
        self.mesh_vertices.clear();
        self.mesh_normals.clear();
        self.mesh_colours.clear();
        self.mesh_indices.clear();
        self.mesh_uvs.clear();
        self.mesh_tangents.clear();

        self.start_path(pos, colour);
    }

    /// Execute a sequence of actions
    pub fn do_actions<A: Into<Action> + Copy>(&mut self, actions: &[A]) {
        let mut depth: usize = 0;
        let mut cut_at: Option<usize> = None;

        for &raw in actions {
            let action = raw.into();
            if let Some(cd) = cut_at {
                // Cutting: track bracket depth without rendering
                match action {
                    Action::Push => depth += 1,
                    Action::Pop => {
                        if depth == cd {
                            cut_at = None;
                            self.pop();
                        }
                        depth = depth.saturating_sub(1);
                    }
                    _ => {}
                }
            } else {
                match action {
                    Action::Push => {
                        depth += 1;
                        self.push();
                    }
                    Action::Pop => {
                        depth = depth.saturating_sub(1);
                        self.pop();
                    }
                    Action::Cut => {
                        if depth > 0 {
                            cut_at = Some(depth);
                        }
                    }
                    other => {
                        let mut branch_segment = None;

                        // Space pruning
                        if self.space_pruning
                            && let Action::Branch(length, diameter) = other
                        {
                            let base = self.pos();
                            let end = base + (length * self.scale) * self.heading;
                            let radius = (diameter * self.scale) * 0.5;

                            // Cut if branch intersects
                            if depth > 0 && self.segment_occupied(base, end, radius) {
                                cut_at = Some(depth);
                                continue;
                            }

                            branch_segment = Some((base, end, radius));
                        }

                        self.do_action(other);

                        // Mark all cells the cylinder occupies
                        if let Some((base, end, radius)) = branch_segment {
                            self.mark_segment(base, end, radius);
                        }
                    }
                }
            }
        }
    }

    pub fn do_action<A: Into<Action> + Copy>(&mut self, action: A) {
        match action.into() {
            Action::Travel(distance) => self.travel(distance),
            Action::Branch(length, diameter) => self.branch(length, diameter),
            Action::Leaf(width, height) => self.leaf(width, height),
            Action::Roll(angle) => self.roll(angle),
            Action::Turn(angle) => self.turn(angle),
            Action::Pitch(angle) => self.pitch(angle),
            Action::Colour(colour) => self.set_colour(colour),
            Action::Push => self.push(),
            Action::Pop => self.pop(),
            Action::Cut | Action::Nop => (),
        };
    }

    /// Pushes the current turtle state to the stack and resets per-branch accumulators
    pub fn push(&mut self) {
        self.stack.push(StackEntry {
            entry_idx: self.path_entries.len() - 1,
            normal: self.normal,
            heading: self.heading,
            prop_curvature: self.prop_curvature,
            prop_bend_axis: self.prop_bend_axis,
            ou_drift: self.ou_drift,
            branch_arc_length: self.branch_arc_length,
            colour_drift: self.colour_drift,
        });
        // Reset per-branch accumulators for the child branch
        self.prop_curvature = 0.0;
        self.prop_bend_axis = Vec3::Y;
        self.ou_drift = Vec3::ZERO;
        self.branch_arc_length = 0.0;
        self.colour_drift = Vec3::ZERO;
    }

    /// Pops the last turtle state from the stack, restoring position, heading, and accumulators
    pub fn pop(&mut self) {
        if let Some(saved) = self.stack.pop() {
            let prev = self.path_entries[saved.entry_idx];
            self.path_entries.push(PathEntry {
                jump: true,
                path_idx: prev.path_idx,
                colour_idx: prev.colour_idx,
            });
            self.normal = saved.normal;
            self.heading = saved.heading;
            self.prop_curvature = saved.prop_curvature;
            self.prop_bend_axis = saved.prop_bend_axis;
            self.ou_drift = saved.ou_drift;
            self.branch_arc_length = saved.branch_arc_length;
            self.colour_drift = saved.colour_drift;
        }
    }

    fn pos(&self) -> Vec3 {
        self.path_buf[self.path_entries.last().unwrap().path_idx]
    }

    /// Move `distance` along the current heading, applying all active environmental effects.
    ///
    /// Order of operations:
    /// 1. Advance position using the current heading (before any deflection)
    /// 2. Phototropism and gravitropism
    /// 3. Wind deflection
    /// 4. Proprioceptive correction
    /// 5. Heading drift
    /// 6. Increment arc-length accumulator
    pub fn travel(&mut self, distance: f32) {
        let heading_before = self.heading;

        // 1. Advance position
        let entry = PathEntry {
            jump: false,
            path_idx: self.path_buf.len(),
            colour_idx: self.colour_buf.len() - 1,
        };
        self.path_buf
            .push(self.pos() + (distance * self.scale) * self.heading);
        self.path_entries.push(entry);

        // 2. Phototropism and gravitropism
        self.apply_tropism_toward(self.tropism_direction, self.tropism_strength);
        self.apply_tropism_toward(Vec3::Y, self.gravitropism_strength);

        // 3. Wind
        self.apply_tropism_toward(self.wind_direction, self.wind_strength);
        self.apply_wind_turbulence();

        // 4. Proprioceptive correction
        self.apply_proprioception(heading_before, distance);

        // 5. Heading drift
        self.apply_ou_variation();

        // 6. Arc-length accumulator for taper
        self.branch_arc_length += distance;
    }

    /// Generate a cylinder mesh and advance the turtle
    pub fn branch(&mut self, length: f32, diameter: f32) {
        // Arc-length taper: full diameter at arc_length=0, decreasing with distance
        let tapered_diameter = diameter * self.taper.powf(self.branch_arc_length);
        let radius = (tapered_diameter * self.scale) * 0.5;

        // LOD: skip mesh for very thin branches or when segments == 0
        if self.lod_segments == 0 || (self.lod_min_radius > 0.0 && radius < self.lod_min_radius) {
            self.travel(length);
            return;
        }

        let base_pos = self.pos();
        let base_colour = *self.colour_buf.last().unwrap();
        let base_idx = self.mesh_vertices.len() as u32;

        let tip_pos = base_pos + (length * self.scale) * self.heading;
        let binormal = self.heading.cross(self.normal).normalize();

        // Apply colour drift to cylinder colour (clamped to valid range)
        let colour = (base_colour + self.colour_drift).clamp(Vec3::ZERO, Vec3::ONE);

        for i in 0..self.lod_segments {
            let angle = (i as f32 / self.lod_segments as f32) * TAU;
            let (sin, cos) = angle.sin_cos();
            let radial = cos * self.normal + sin * binormal;
            let offset = radial * radius;

            // Base vertex
            self.mesh_vertices.push(base_pos + offset);
            self.mesh_normals.push(radial);
            self.mesh_colours.push(colour);
            self.mesh_uvs.push([-1.0, -1.0]);
            self.mesh_tangents.push(Vec3::ZERO);

            // Tip vertex
            self.mesh_vertices.push(tip_pos + offset);
            self.mesh_normals.push(radial);
            self.mesh_colours.push(colour);
            self.mesh_uvs.push([-1.0, -1.0]);
            self.mesh_tangents.push(Vec3::ZERO);
        }

        // Generate quad indices for each segment (2 triangles per quad)
        for i in 0..self.lod_segments as u32 {
            let next = (i + 1) % self.lod_segments as u32;
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
        if self.leaf_skip_prob > 0.0 && rng::random_range(0f32, 1.0) < self.leaf_skip_prob {
            return;
        }

        let base_idx = self.mesh_vertices.len() as u32;
        let pos = self.pos();

        let binormal = self.heading.cross(self.normal).normalize();
        let colour =
            (*self.colour_buf.last().unwrap() + self.colour_drift).clamp(Vec3::ZERO, Vec3::ONE);

        let half_w = (width * self.scale) * 0.5;
        let bl = pos - binormal * half_w;
        let br = pos + binormal * half_w;
        let tl = pos - binormal * half_w + self.heading * height * self.scale;
        let tr = pos + binormal * half_w + self.heading * height * self.scale;

        // Pick a random cell from the 3×3 leaf atlas
        let cell = rng::random_range(0.0, 9.0) as u32;
        let col = (cell % 3) as f32;
        let row = (cell / 3) as f32;
        let u0 = col / 3.0;
        let u1 = (col + 1.0) / 3.0;
        let v0 = row / 3.0;
        let v1 = (row + 1.0) / 3.0;

        // Front face vertices
        self.mesh_vertices.extend([bl, br, tl, tr]);
        for _ in 0..4 {
            self.mesh_normals.push(self.normal);
            self.mesh_colours.push(colour);
            self.mesh_tangents.push(binormal);
        }
        self.mesh_uvs
            .extend([[u0, v1], [u1, v1], [u0, v0], [u1, v0]]);

        // Back face vertices
        self.mesh_vertices.extend([bl, br, tl, tr]);
        for _ in 0..4 {
            self.mesh_normals.push(-self.normal);
            self.mesh_colours.push(colour);
            self.mesh_tangents.push(binormal);
        }
        self.mesh_uvs
            .extend([[u0, v1], [u1, v1], [u0, v0], [u1, v0]]);

        self.mesh_indices.extend([
            // Front face triangles
            base_idx,
            base_idx + 2,
            base_idx + 1,
            base_idx + 1,
            base_idx + 2,
            base_idx + 3,
            // Back face triangles
            base_idx + 4,
            base_idx + 5,
            base_idx + 6,
            base_idx + 5,
            base_idx + 7,
            base_idx + 6,
        ]);
    }

    /// Turn clockwise by `angle` around current normal
    pub fn turn(&mut self, angle: f32) {
        self.heading = self.heading.rotate_axis(self.normal, angle).normalize();
    }

    /// Roll clockwise by `angle` around current heading
    pub fn roll(&mut self, angle: f32) {
        self.normal = self.normal.rotate_axis(self.heading, angle).normalize();
    }

    /// Pitch by `angle` around the binormal axis (heading × normal)
    pub fn pitch(&mut self, angle: f32) {
        let binormal = self.heading.cross(self.normal).normalize_or_zero();
        if binormal.length_squared() > 1e-12 {
            self.heading = self.heading.rotate_axis(binormal, angle).normalize();
            self.normal = self.normal.rotate_axis(binormal, angle).normalize();
        }
    }

    /// Set the line colour
    pub fn set_colour(&mut self, colour: Vec3) {
        self.colour_buf.push(colour);
    }

    /// Set the scale
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }

    /// Set the phototrophism direction and strength
    pub fn set_tropism(&mut self, direction: Vec3, strength: f32) {
        self.tropism_direction = direction;
        self.tropism_strength = strength;
    }

    /// Set the gravitropism strength
    pub fn set_gravitropism(&mut self, strength: f32) {
        self.gravitropism_strength = strength;
    }

    /// Sets the wind direction (normalised horizontal vector), directional strength,
    /// and turbulence (max random rotation in radians per travel step)
    pub fn set_wind(&mut self, direction: Vec3, strength: f32, turbulence: f32) {
        self.wind_direction = direction;
        self.wind_strength = strength;
        self.wind_turbulence = turbulence;
    }

    /// Controls whether wind deflection is baked into geometry
    pub fn set_wind_baked(&mut self, baked: bool) {
        self.wind_baked = baked;
    }

    /// Sets the taper applied per unit of arc-length along a branch
    /// `1.0` = uniform width; `0.7` = branch shrinks to 70% diameter per unit arc-length
    pub fn set_taper(&mut self, taper: f32) {
        self.taper = taper;
    }

    /// Set LOD parameters for this turtle
    /// - `segments` controls cylinder quality (0 = skip mesh generation entirely)
    /// - `min_radius` skips branches thinner than this value
    pub fn set_lod(&mut self, segments: usize, min_radius: f32) {
        self.lod_segments = segments;
        self.lod_min_radius = min_radius;
    }

    /// Set the probability that any individual leaf quad is skipped
    pub fn set_leaf_skip_prob(&mut self, prob: f32) {
        self.leaf_skip_prob = prob.clamp(0.0, 1.0);
    }

    /// Set the proprioceptive self-correction rate
    pub fn set_proprioception(&mut self, gamma: f32) {
        self.proprioception_gamma = gamma;
    }

    /// Set heading drift parameters
    /// - `noise_strength` controls the per-step noise amplitude
    /// - `decay_rate` ∈ \[0,1\] controls how quickly the drift returns to zero (θ)
    /// - `colour_variation` controls per-branch colour drift amplitude
    pub fn set_variation(&mut self, noise_strength: f32, decay_rate: f32, colour_variation: f32) {
        self.variation_noise_strength = noise_strength;
        self.variation_decay_rate = decay_rate.clamp(0.0, 1.0);
        self.colour_variation = colour_variation;
    }

    /// Enable voxel-grid space pruning
    pub fn set_space_pruning(&mut self, enabled: bool, cell_size: f32) {
        self.space_pruning = enabled;
        self.occupancy_cell_size = cell_size.max(0.01);
    }

    /// Clear both occupancy sets
    pub fn clear_occupancy(&mut self) {
        self.occupancy_grid.clear();
        self.current_plant_marks.clear();
    }

    /// Applies correlated random drift to heading and colour
    /// - drift = drift * (1 - θ) + noise * σ
    fn apply_ou_variation(&mut self) {
        if self.variation_noise_strength <= 0.0 {
            return;
        }

        let noise = Vec3::new(
            rng::random_range(-1.0, 1.0),
            rng::random_range(-1.0, 1.0),
            rng::random_range(-1.0, 1.0),
        );
        self.ou_drift = self.ou_drift * (1.0 - self.variation_decay_rate)
            + noise * self.variation_noise_strength;

        let drift_len = self.ou_drift.length();
        if drift_len > 1e-6 {
            let axis = self.ou_drift / drift_len;
            // Cap rotation angle to avoid wild heading flips
            let angle = drift_len.min(0.5);
            self.heading = self.heading.rotate_axis(axis, angle).normalize();
            self.normal = self.normal.rotate_axis(axis, angle).normalize();
        }

        // Colour OU drift (independent process)
        if self.colour_variation > 0.0 {
            let cn = Vec3::new(
                rng::random_range(-1.0, 1.0),
                rng::random_range(-1.0, 1.0),
                rng::random_range(-1.0, 1.0),
            );
            self.colour_drift =
                self.colour_drift * (1.0 - self.variation_decay_rate) + cn * self.colour_variation;
        }
    }

    /// Apply a random rotation in the plane perpendicular to heading to simulate wind gusts
    fn apply_wind_turbulence(&mut self) {
        if self.wind_turbulence <= 0.0 {
            return;
        }

        let angle = rng::random_range(-self.wind_turbulence, self.wind_turbulence);
        let roll = rng::random_range(0.0, TAU);

        let binormal = self.heading.cross(self.normal).normalize_or_zero();
        let perp = self.normal * roll.cos() + binormal * roll.sin();

        if perp.length_squared() > 1e-12 {
            let axis = perp.normalize();
            self.heading = self.heading.rotate_axis(axis, angle).normalize();
            self.normal = self.normal.rotate_axis(axis, angle).normalize();
        }
    }

    /// Rotate heading toward `direction` by an amount proportional to `strength`
    fn apply_tropism_toward(&mut self, direction: Vec3, strength: f32) {
        if strength <= 0.0 {
            return;
        }

        let torque = self.heading.cross(direction);
        let torque_len = torque.length();

        if torque_len > 1e-6 {
            let axis = torque / torque_len;
            let angle = strength * torque_len;
            self.heading = self.heading.rotate_axis(axis, angle).normalize();
            self.normal = self.normal.rotate_axis(axis, angle).normalize();
        }
    }

    /// Discrete proprioceptive correction (straightening)
    fn apply_proprioception(&mut self, heading_before: Vec3, length: f32) {
        if self.proprioception_gamma <= 0.0 {
            return;
        }

        let (bend_axis, bend_mag) = heading_before.cross(self.heading).normalize_and_length();
        if bend_mag > 1e-6 {
            // Accumulate curvature weighted by segment length
            self.prop_curvature += bend_mag * length;
            self.prop_bend_axis = bend_axis;
        }

        // Apply straightening correction opposite to curvature
        let correction = self.proprioception_gamma * self.prop_curvature;
        if correction > 1e-6 && self.prop_bend_axis.length_squared() > 1e-12 {
            self.heading = self
                .heading
                .rotate_axis(self.prop_bend_axis, -correction)
                .normalize();
            self.normal = self
                .normal
                .rotate_axis(self.prop_bend_axis, -correction)
                .normalize();
        }

        // Decay curvature signal
        self.prop_curvature *= 1.0 - self.proprioception_gamma.min(0.99);
    }

    /// Convert a world-space position to voxel grid coordinates
    fn world_to_cell(&self, pos: Vec3) -> (i32, i32, i32) {
        let s = 1.0 / self.occupancy_cell_size;
        (
            (pos.x * s).floor() as i32,
            (pos.y * s).floor() as i32,
            (pos.z * s).floor() as i32,
        )
    }

    /// Returns true if any voxel within `radius` of the segment `start`→`end` was occupied
    /// by a previously completed plant. Steps at `occupancy_cell_size` intervals along the
    /// axis and expands each sample by `ceil(radius / cell_size)` cells in every direction,
    /// approximating the cylinder cross-section with an AABB.
    /// When `radius < cell_size` the expansion is zero and this reduces to a centre-line check.
    fn segment_occupied(&self, start: Vec3, end: Vec3, radius: f32) -> bool {
        let seg = end - start;

        let steps = (seg.length() / self.occupancy_cell_size).ceil().max(1.0);
        let r = (radius / self.occupancy_cell_size).ceil() as i32;

        (0..=steps as usize).any(|i| {
            let (cx, cy, cz) = self.world_to_cell(start + (i as f32 / steps) * seg);
            (-r..=r).any(|dx| {
                (-r..=r).any(|dy| {
                    (-r..=r).any(|dz| self.occupancy_grid.contains(&(cx + dx, cy + dy, cz + dz)))
                })
            })
        })
    }

    /// Marks every voxel within `radius` of the segment `start`→`end` as occupied by the
    /// current plant, using the same AABB expansion as `segment_occupied`.
    fn mark_segment(&mut self, start: Vec3, end: Vec3, radius: f32) {
        let seg = end - start;
        let len = seg.length();
        let steps = (len / self.occupancy_cell_size).ceil().max(1.0);
        let r = (radius / self.occupancy_cell_size).ceil() as i32;

        for i in 0..=steps as usize {
            let (cx, cy, cz) = self.world_to_cell(start + (i as f32 / steps) * seg);
            for dx in -r..=r {
                for dy in -r..=r {
                    for dz in -r..=r {
                        self.current_plant_marks.insert((cx + dx, cy + dy, cz + dz));
                    }
                }
            }
        }
    }

    /// Extract line geometry data without writing to GPU
    pub fn line_geometry(&self) -> LineGeometry {
        let mut vertices = Vec::new();
        let mut colours = Vec::new();
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
                    colours.push(self.colour_buf[entry.colour_idx]);
                    indices.push(vertex_count);
                    vertex_count += 1;
                    segment_length += 1;
                }

                vertices.push(self.path_buf[entry.path_idx]);
                colours.push(self.colour_buf[entry.colour_idx]);
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
            colours,
            indices,
            segments,
        }
    }

    /// Number of mesh indices currently accumulated
    pub fn mesh_index_count(&self) -> u32 {
        self.mesh_indices.len() as u32
    }

    /// Extract mesh geometry data without writing to GPU
    pub fn mesh_geometry(&self) -> MeshGeometry {
        MeshGeometry {
            vertices: self.mesh_vertices.clone(),
            normals: self.mesh_normals.clone(),
            colours: self.mesh_colours.clone(),
            indices: self.mesh_indices.clone(),
            uvs: self.mesh_uvs.clone(),
            tangents: self.mesh_tangents.clone(),
        }
    }

    /// Move mesh geometry out of the turtle (zero-copy). Leaves the turtle's mesh buffers empty.
    pub fn take_mesh_geometry(&mut self) -> MeshGeometry {
        MeshGeometry {
            vertices: std::mem::take(&mut self.mesh_vertices),
            normals: std::mem::take(&mut self.mesh_normals),
            colours: std::mem::take(&mut self.mesh_colours),
            indices: std::mem::take(&mut self.mesh_indices),
            uvs: std::mem::take(&mut self.mesh_uvs),
            tangents: std::mem::take(&mut self.mesh_tangents),
        }
    }
}

// ── Geometry data structures ──

#[derive(Clone)]
pub struct LineGeometry {
    pub vertices: Vec<Vec3>,
    pub colours: Vec<Vec3>,
    pub indices: Vec<u32>,
    pub segments: Vec<(u32, u32)>,
}

#[derive(Clone)]
pub struct MeshGeometry {
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub colours: Vec<Vec3>,
    pub indices: Vec<u32>,
    pub uvs: Vec<[f32; 2]>,
    pub tangents: Vec<Vec3>,
}

/// Combine multiple line geometries into one, adjusting indices
pub fn combine_line_geometries(geos: &[&LineGeometry]) -> LineGeometry {
    let total_verts: usize = geos.iter().map(|g| g.vertices.len()).sum();
    let total_indices: usize = geos.iter().map(|g| g.indices.len()).sum();
    let total_segments: usize = geos.iter().map(|g| g.segments.len()).sum();

    let mut vertices = Vec::with_capacity(total_verts);
    let mut colours = Vec::with_capacity(total_verts);
    let mut indices = Vec::with_capacity(total_indices);
    let mut segments = Vec::with_capacity(total_segments);

    for geo in geos {
        let vertex_offset = vertices.len() as u32;
        let index_offset = indices.len() as u32;

        vertices.extend_from_slice(&geo.vertices);
        colours.extend_from_slice(&geo.colours);
        indices.extend(geo.indices.iter().map(|i| i + vertex_offset));
        segments.extend(
            geo.segments
                .iter()
                .map(|(start, count)| (start + index_offset, *count)),
        );
    }

    LineGeometry {
        vertices,
        colours,
        indices,
        segments,
    }
}

/// Combine multiple mesh geometries into one, adjusting indices
pub fn combine_mesh_geometries(geos: &[&MeshGeometry]) -> MeshGeometry {
    let total_verts: usize = geos.iter().map(|g| g.vertices.len()).sum();
    let total_indices: usize = geos.iter().map(|g| g.indices.len()).sum();

    let mut vertices = Vec::with_capacity(total_verts);
    let mut normals = Vec::with_capacity(total_verts);
    let mut colours = Vec::with_capacity(total_verts);
    let mut indices = Vec::with_capacity(total_indices);
    let mut uvs = Vec::with_capacity(total_verts);
    let mut tangents = Vec::with_capacity(total_verts);

    for geo in geos {
        let vertex_offset = vertices.len() as u32;

        vertices.extend_from_slice(&geo.vertices);
        normals.extend_from_slice(&geo.normals);
        colours.extend_from_slice(&geo.colours);
        indices.extend(geo.indices.iter().map(|i| i + vertex_offset));
        uvs.extend_from_slice(&geo.uvs);
        tangents.extend_from_slice(&geo.tangents);
    }

    MeshGeometry {
        vertices,
        normals,
        colours,
        indices,
        uvs,
        tangents,
    }
}

#[cfg(test)]
mod tests {
    use glam::{Vec3, vec3};

    use super::*;

    fn white_turtle() -> Turtle {
        Turtle::new(Vec3::ZERO, Vec3::ONE)
    }

    // ── Line geometry ──

    #[test]
    fn fresh_turtle_has_no_line_segments() {
        let t = white_turtle();
        let geo = t.line_geometry();
        assert!(geo.segments.is_empty());
        assert!(geo.indices.is_empty());
    }

    #[test]
    fn travel_produces_one_segment() {
        let mut t = white_turtle();
        t.travel(1.0);
        let geo = t.line_geometry();
        assert_eq!(geo.segments.len(), 1);
        assert_eq!(geo.vertices.len(), 2);
        // Default heading is Vec3::X, so endpoint is (1, 0, 0)
        assert!((geo.vertices[1] - vec3(1.0, 0.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn push_pop_restores_position_for_travel() {
        let mut t = white_turtle();
        t.travel(1.0);
        t.push();
        t.turn(std::f32::consts::FRAC_PI_2);
        t.travel(5.0); // travels in a different direction
        t.pop();
        t.travel(2.0); // should continue from (1,0,0) along Vec3::X
        let geo = t.line_geometry();
        // Last vertex of the main branch is at (3, 0, 0)
        let last = *geo.vertices.last().unwrap();
        assert!((last - vec3(3.0, 0.0, 0.0)).length() < 1e-4);
    }

    #[test]
    fn combine_line_geometries_merges_indices() {
        let mut t1 = white_turtle();
        t1.travel(1.0);
        let mut t2 = white_turtle();
        t2.travel(2.0);
        let geo1 = t1.line_geometry();
        let geo2 = t2.line_geometry();
        let combined = combine_line_geometries(&[&geo1, &geo2]);
        assert_eq!(combined.vertices.len(), 4); // 2 + 2
        assert_eq!(combined.segments.len(), 2);
    }

    // ── Mesh geometry ──

    #[test]
    fn fresh_turtle_has_no_mesh() {
        let t = white_turtle();
        let geo = t.mesh_geometry();
        assert!(geo.vertices.is_empty());
        assert!(geo.indices.is_empty());
    }

    #[test]
    fn branch_generates_cylinder_geometry() {
        let mut t = white_turtle();
        t.branch(1.0, 0.1);
        let geo = t.mesh_geometry();
        // Default 8 segments: base ring (8) + tip ring (8) = 16 vertices
        // Each of 8 segments produces 2 triangles = 6 indices each → 48 total
        assert_eq!(geo.vertices.len(), 16);
        assert_eq!(geo.indices.len(), 48);
    }

    #[test]
    fn branch_lod_zero_segments_skips_mesh() {
        let mut t = white_turtle();
        t.set_lod(0, 0.0);
        t.branch(1.0, 0.1);
        assert!(t.mesh_geometry().vertices.is_empty());
    }

    #[test]
    fn branch_below_min_radius_skips_mesh() {
        let mut t = white_turtle();
        t.set_lod(8, 1.0); // min_radius = 1.0, branch radius ≈ 0.05
        t.branch(1.0, 0.1);
        assert!(t.mesh_geometry().vertices.is_empty());
    }

    #[test]
    fn leaf_generates_double_sided_quad() {
        let mut t = white_turtle();
        t.leaf(0.5, 1.0);
        let geo = t.mesh_geometry();
        // Front face: 4 verts, back face: 4 verts = 8 total
        // Front: 6 indices, back: 6 indices = 12 total
        assert_eq!(geo.vertices.len(), 8);
        assert_eq!(geo.indices.len(), 12);
    }

    #[test]
    fn multiple_leaves_accumulate() {
        let mut t = white_turtle();
        t.leaf(0.5, 1.0);
        t.leaf(0.5, 1.0);
        let geo = t.mesh_geometry();
        assert_eq!(geo.vertices.len(), 16);
        assert_eq!(geo.indices.len(), 24);
    }

    // ── Cut symbol ──

    #[test]
    fn cut_inside_branch_skips_to_pop() {
        let mut t = white_turtle();
        t.do_actions(&[
            Action::Push,
            Action::Cut,
            Action::Leaf(0.5, 1.0), // should be skipped
            Action::Pop,
        ]);
        // Leaf was skipped, so no mesh
        assert!(t.mesh_geometry().vertices.is_empty());
    }

    #[test]
    fn cut_at_depth_zero_is_ignored() {
        let mut t = white_turtle();
        t.do_actions(&[
            Action::Cut,            // no effect at depth 0
            Action::Leaf(0.5, 1.0), // should execute normally
        ]);
        assert_eq!(t.mesh_geometry().vertices.len(), 8);
    }

    #[test]
    fn cut_only_affects_enclosing_branch() {
        let mut t = white_turtle();
        t.do_actions(&[
            Action::Push,
            Action::Cut,
            Action::Leaf(0.5, 1.0), // skipped
            Action::Pop,
            Action::Leaf(0.5, 1.0), // should still execute
        ]);
        assert_eq!(t.mesh_geometry().vertices.len(), 8);
    }

    // ── Space pruning (voxel occupancy) ──

    #[test]
    fn space_pruning_intra_plant_branches_never_block_each_other() {
        // Within a single plant build, branches should not prune each other even if they
        // head in the same direction — intra-plant marks live in current_plant_marks which
        // is not checked during is_cell_occupied().
        let mut t = white_turtle();
        t.set_space_pruning(true, 0.5);
        t.do_actions(&[Action::Push, Action::Branch(1.0, 0.1), Action::Pop]);
        let first_count = t.mesh_geometry().indices.len();
        assert!(first_count > 0);
        // Second identical branch from same plant — should NOT be pruned
        t.do_actions(&[Action::Push, Action::Branch(1.0, 0.1), Action::Pop]);
        assert_eq!(t.mesh_geometry().indices.len(), first_count * 2);
    }

    #[test]
    fn space_pruning_inter_plant_branches_are_blocked() {
        // After reset(), the first plant's marks are flushed into occupancy_grid.
        // A second plant building into the same voxels should be pruned.
        let mut t = white_turtle();
        t.set_space_pruning(true, 0.5);
        // Plant 1
        t.do_actions(&[Action::Push, Action::Branch(1.0, 0.1), Action::Pop]);
        let after_p1 = t.mesh_geometry().indices.len();
        assert!(after_p1 > 0);
        // Simulate next plant: reset() flushes plant 1 marks into grid
        t.reset(Vec3::ZERO, Vec3::ONE);
        t.set_space_pruning(true, 0.5);
        // Plant 2 — same direction, same voxel — should be pruned
        t.do_actions(&[Action::Push, Action::Branch(1.0, 0.1), Action::Pop]);
        // No new mesh was added (plant 2's branch was cut)
        assert_eq!(t.mesh_geometry().indices.len(), 0);
    }

    #[test]
    fn space_pruning_blocks_mid_segment_overlap() {
        // Plant 1 builds a short branch occupying cells 0..1 along X.
        // Plant 2 starts further along X so its TIP doesn't overlap, but its
        // mid-segment passes through a cell plant 1 marked.
        // With tip-only checking this would not be caught; with full-segment
        // checking it should be pruned.
        let mut t = white_turtle();
        t.set_space_pruning(true, 1.0); // 1-unit cells
        // Plant 1: branch from (0,0,0) in +X direction, length 1 → marks cell (0,0,0)
        t.do_actions(&[Action::Push, Action::Branch(1.0, 0.1), Action::Pop]);
        assert!(!t.mesh_geometry().indices.is_empty());

        // Flush plant 1 marks into the grid
        t.reset(Vec3::ZERO, Vec3::ONE);
        t.set_space_pruning(true, 1.0);
        // Plant 2: start at (-0.5, 0, 0), still heading +X, length 2.
        // Tip lands at (1.5, 0, 0) — cell (1,0,0) which is NOT occupied.
        // But the segment passes through (0,0,0) which IS occupied by plant 1.
        t.reset(Vec3::new(-0.5, 0.0, 0.0), Vec3::ONE);
        t.set_space_pruning(true, 1.0);
        t.do_actions(&[Action::Push, Action::Branch(2.0, 0.1), Action::Pop]);
        // Should be pruned because the mid-segment overlaps plant 1's cell
        assert_eq!(t.mesh_geometry().indices.len(), 0);
    }

    #[test]
    fn space_pruning_considers_branch_width() {
        // Two parallel branches with axes that don't intersect, but whose cylinders overlap.
        // Plant 1: axis along +X at Y=0, diameter=2.0 → radius=1.0, cell_size=1.0 → r_cells=1.
        //   Marks cells within 1 cell of the axis, including Y=1 cells.
        // Plant 2: axis along +X at Y=0.8 (offset in Y). Without width expansion the axes
        //   don't share a cell; with width expansion they do.
        let mut t = white_turtle();
        t.set_space_pruning(true, 1.0);
        // Plant 1: fat branch (diameter 2.0 → radius 1.0) along +X at origin
        t.do_actions(&[Action::Push, Action::Branch(1.0, 2.0), Action::Pop]);
        assert!(!t.mesh_geometry().indices.is_empty());

        // Flush plant 1 marks
        t.reset(Vec3::ZERO, Vec3::ONE);
        // Plant 2: thin branch at Y=0.8 (different axis, tips at different cells).
        // Without width expansion: axis at Y=0.8 falls in cell Y=0, same cell — still blocked.
        // So use Y=1.2 to ensure the axis is in a different cell (Y=1), but within radius of plant 1.
        t.reset(Vec3::new(0.0, 1.2, 0.0), Vec3::ONE);
        t.set_space_pruning(true, 1.0);
        t.do_actions(&[Action::Push, Action::Branch(1.0, 2.0), Action::Pop]);
        // Plant 1's radius expansion marks Y=1 cells; plant 2's base radius also expands.
        // Plant 2 should be blocked because plant 1's marked region overlaps its cylinder.
        assert_eq!(t.mesh_geometry().indices.len(), 0);
    }

    #[test]
    fn space_pruning_allows_well_separated_branches() {
        // Branches far enough apart (beyond both radii) should not block each other.
        let mut t = white_turtle();
        t.set_space_pruning(true, 1.0);
        // Plant 1: axis at Y=0, diameter=0.4 → radius=0.2, r_cells=0 (< cell_size).
        t.do_actions(&[Action::Push, Action::Branch(1.0, 0.4), Action::Pop]);
        assert!(!t.mesh_geometry().indices.is_empty());

        t.reset(Vec3::ZERO, Vec3::ONE);
        // Plant 2: axis at Y=3.0 — well outside any marked cell.
        t.reset(Vec3::new(0.0, 3.0, 0.0), Vec3::ONE);
        t.set_space_pruning(true, 1.0);
        t.do_actions(&[Action::Push, Action::Branch(1.0, 0.4), Action::Pop]);
        assert!(
            !t.mesh_geometry().indices.is_empty(),
            "well-separated branch should not be pruned"
        );
    }

    #[test]
    fn space_pruning_disabled_allows_overlapping_branches() {
        let mut t = white_turtle();
        t.set_space_pruning(false, 0.5);
        t.do_actions(&[Action::Push, Action::Branch(1.0, 0.1), Action::Pop]);
        let first_count = t.mesh_geometry().indices.len();
        t.do_actions(&[Action::Push, Action::Branch(1.0, 0.1), Action::Pop]);
        assert_eq!(t.mesh_geometry().indices.len(), first_count * 2);
    }

    // ── Combine mesh geometries ──

    #[test]
    fn combine_mesh_geometries_offsets_indices() {
        let mut t = white_turtle();
        t.leaf(0.5, 1.0); // 8 vertices, indices 0..7
        let geo1 = t.mesh_geometry();

        let mut t2 = white_turtle();
        t2.leaf(0.5, 1.0); // 8 vertices, indices 0..7 (before offset)
        let geo2 = t2.mesh_geometry();

        let combined = combine_mesh_geometries(&[&geo1, &geo2]);
        assert_eq!(combined.vertices.len(), 16);
        // Indices in second geo should be offset by 8
        assert!(combined.indices.iter().all(|&i| i < 16));
        assert!(combined.indices[12..].iter().all(|&i| i >= 8));
    }

    // ── mesh_index_count ──

    #[test]
    fn mesh_index_count_matches_geometry() {
        let mut t = white_turtle();
        t.branch(1.0, 0.1);
        t.leaf(0.5, 1.0);
        let geo = t.mesh_geometry();
        assert_eq!(t.mesh_index_count(), geo.indices.len() as u32);
    }

    // ── Orientation: turn / roll / pitch ──

    #[test]
    fn turn_rotates_heading_left() {
        // Default: heading=+X, normal=+Y.
        // turn(PI/2) rotates heading around +Y by PI/2 → heading becomes -Z.
        // Travelling then moves to (0, 0, -1).
        let mut t = white_turtle();
        t.turn(std::f32::consts::FRAC_PI_2);
        t.travel(1.0);
        let geo = t.line_geometry();
        let tip = geo.vertices[1];
        assert!((tip - vec3(0.0, 0.0, -1.0)).length() < 1e-5, "tip={tip:?}");
    }

    #[test]
    fn roll_does_not_change_heading() {
        // roll rotates the normal around the heading — heading itself is unchanged.
        let mut t = white_turtle();
        t.roll(std::f32::consts::FRAC_PI_2);
        t.travel(1.0);
        let geo = t.line_geometry();
        let tip = geo.vertices[1];
        assert!((tip - vec3(1.0, 0.0, 0.0)).length() < 1e-5, "tip={tip:?}");
    }

    #[test]
    fn pitch_tilts_heading_upward() {
        // Default: heading=+X, normal=+Y, binormal=X×Y=+Z.
        // pitch(PI/2) rotates heading (and normal) around +Z by PI/2 → heading=+Y.
        let mut t = white_turtle();
        t.pitch(std::f32::consts::FRAC_PI_2);
        t.travel(1.0);
        let geo = t.line_geometry();
        let tip = geo.vertices[1];
        assert!((tip - vec3(0.0, 1.0, 0.0)).length() < 1e-5, "tip={tip:?}");
    }

    // ── Scale ──

    #[test]
    fn scale_multiplies_travel_distance() {
        let mut t = white_turtle();
        t.set_scale(2.0);
        t.travel(1.0);
        let geo = t.line_geometry();
        let tip = geo.vertices[1];
        assert!((tip - vec3(2.0, 0.0, 0.0)).length() < 1e-5, "tip={tip:?}");
    }

    // ── LOD segment count ──

    #[test]
    fn lod_segment_count_controls_branch_vertices() {
        let mut t = white_turtle();
        t.set_lod(4, 0.0);
        t.branch(1.0, 0.1);
        let geo = t.mesh_geometry();
        // 4 segments × 2 verts (base+tip) = 8 vertices; 4 × 6 indices = 24
        assert_eq!(geo.vertices.len(), 8, "expected 8 verts for 4 segments");
        assert_eq!(geo.indices.len(), 24, "expected 24 indices for 4 segments");
    }

    // ── Nested push/pop ──

    #[test]
    fn nested_push_pop_restores_to_outer_state() {
        // travel(1) → (1,0,0)
        // push; turn(PI/2) → heading=-Z
        //   push; turn(PI/2) → heading=-X; travel(5) → (-4,0,0)
        //   pop  → restored to (1,0,0), heading=-Z
        //   travel(1) → (1,0,-1)
        // pop  → restored to (1,0,0), heading=+X
        // travel(2) → (3,0,0)
        let mut t = white_turtle();
        t.travel(1.0);
        t.push();
        t.turn(std::f32::consts::FRAC_PI_2); // heading → -Z
        t.push();
        t.turn(std::f32::consts::FRAC_PI_2); // heading → -X
        t.travel(5.0);
        t.pop();
        t.travel(1.0); // should go (1,0,0)→(1,0,-1)
        t.pop();
        t.travel(2.0); // should go (1,0,0)→(3,0,0)
        let geo = t.line_geometry();
        let last = *geo.vertices.last().unwrap();
        assert!(
            (last - vec3(3.0, 0.0, 0.0)).length() < 1e-4,
            "last={last:?}"
        );
    }

    // ── Leaf skip probability ──

    #[test]
    fn leaf_skip_prob_one_skips_all_leaves() {
        // Seeded deterministically — but prob=1.0 means random_range(0,1) < 1.0 is
        // always true, so every leaf is skipped regardless of RNG.
        use crate::util::rng;
        rng::seed(0);
        let mut t = white_turtle();
        t.set_leaf_skip_prob(1.0);
        for _ in 0..10 {
            t.leaf(0.5, 1.0);
        }
        assert!(t.mesh_geometry().vertices.is_empty());
    }
}
