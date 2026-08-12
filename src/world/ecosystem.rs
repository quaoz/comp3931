use std::f32::consts::TAU;

use glam::{Vec2, vec3};

use crate::{
    settings::{EcosystemKernel, EcosystemSettings, PlantType},
    util::rng,
    world::plants::PlantInstance,
};

const GRID_SIZE: usize = 32;

// ── Density grid for kernel-based placement ──
// Implements the spatial distribution algorithm from:
// "Generating Spatial Distributions for Multilevel Models of Plant Communities" (§4)

struct DensityGrid {
    cells: Vec<f32>,
    area: f32,
}

impl DensityGrid {
    fn new(area: f32) -> Self {
        Self {
            cells: vec![1.0; GRID_SIZE * GRID_SIZE],
            area,
        }
    }

    fn normalize(&mut self) {
        let sum: f32 = self.cells.iter().sum();
        if sum > 0.0 {
            for c in &mut self.cells {
                *c /= sum;
            }
        }
    }

    /// Sample a 2D world-space position from the current PDF via inverse transform sampling.
    fn sample(&self) -> Vec2 {
        let total: f32 = self.cells.iter().sum();
        let r = rng::random_range(0.0, total);
        let mut acc = 0.0;
        let mut idx = GRID_SIZE * GRID_SIZE - 1;

        for (i, &v) in self.cells.iter().enumerate() {
            acc += v;
            if acc >= r {
                idx = i;
                break;
            }
        }

        let gy = idx / GRID_SIZE;
        let gx = idx % GRID_SIZE;
        let cx = (gx as f32 / GRID_SIZE as f32 - 0.5) * self.area;
        let cz = (gy as f32 / GRID_SIZE as f32 - 0.5) * self.area;
        let cell_size = self.area / GRID_SIZE as f32;
        let jx = rng::random_range(-cell_size * 0.5, cell_size * 0.5);
        let jz = rng::random_range(-cell_size * 0.5, cell_size * 0.5);
        Vec2::new(cx + jx, cz + jz)
    }

    /// Apply deformation kernel κ centred at (px, pz) to update the PDF.
    fn apply_kernel(&mut self, px: f32, pz: f32, kernel: EcosystemKernel, radius: f32) {
        for gy in 0..GRID_SIZE {
            for gx in 0..GRID_SIZE {
                let wx = (gx as f32 / GRID_SIZE as f32 - 0.5) * self.area;
                let wz = (gy as f32 / GRID_SIZE as f32 - 0.5) * self.area;
                let dist = ((wx - px).powi(2) + (wz - pz).powi(2)).sqrt();
                let r = (dist / radius).min(1.0);
                let factor = match kernel {
                    EcosystemKernel::Neutral => 1.0,
                    EcosystemKernel::Inhibitory => 1.0 - bell(r),
                    EcosystemKernel::Promotional => 1.0 + bell(r),
                    EcosystemKernel::Mixed => 1.0 + mixed_kappa(r),
                };
                let cell = &mut self.cells[gy * GRID_SIZE + gx];
                *cell = (*cell * factor).max(1e-4);
            }
        }
        self.normalize();
    }
}

/// Bell curve kernel: 1 at r=0, decays to ~0 at r=1.
fn bell(r: f32) -> f32 {
    (-r * r * 5.0).exp()
}

/// Mixed kernel: inhibit close range (r < 0.4), weakly promote medium range.
fn mixed_kappa(r: f32) -> f32 {
    if r < 0.4 {
        -bell(r / 0.4)
    } else {
        0.4 * bell((r - 0.7) / 0.3)
    }
}

// ── Plant circles for thinning / succession ──

#[derive(Clone)]
struct PlantCircle {
    pos: Vec2,
    radius: f32,
    plant_type: PlantType,
}

/// Self-thinning: iteratively remove the smaller plant from any overlapping pair
/// until no overlaps remain, producing a regular (overdispersed) distribution.
/// Based on §3.2 of the multilevel plant community paper.
fn self_thinning(circles: &mut Vec<PlantCircle>, thinning_radius: f32) {
    let mut changed = true;
    while changed {
        let n = circles.len();
        let mut to_remove = vec![false; n];
        for i in 0..n {
            for j in (i + 1)..n {
                if to_remove[i] || to_remove[j] {
                    continue;
                }
                let dist = (circles[i].pos - circles[j].pos).length();
                if dist < thinning_radius {
                    if circles[i].radius <= circles[j].radius {
                        to_remove[i] = true;
                    } else {
                        to_remove[j] = true;
                    }
                }
            }
        }
        changed = to_remove.iter().any(|&r| r);
        if changed {
            let mut idx = 0;
            circles.retain(|_| {
                let keep = !to_remove[idx];
                idx += 1;
                keep
            });
        }
    }
}

/// Two-species succession: plants grow each step, overlapping plants compete
/// by shade tolerance (from PlantType), and survivors propagate offspring.
/// Based on §3.3 of the multilevel plant community paper.
fn succession(circles: &mut Vec<PlantCircle>, settings: &EcosystemSettings) {
    let half = settings.area * 0.5;
    for _ in 0..settings.succession_steps {
        // Grow — rate comes from each plant's intrinsic growth_rate()
        for c in circles.iter_mut() {
            c.radius = (c.radius + c.plant_type.growth_rate()).min(settings.kernel_radius * 0.4);
        }

        // Competition: higher shade_tolerance() wins in overlapping pairs
        let n = circles.len();
        let mut to_remove = vec![false; n];
        for i in 0..n {
            for j in (i + 1)..n {
                if to_remove[i] || to_remove[j] {
                    continue;
                }
                let dist = (circles[i].pos - circles[j].pos).length();
                if dist < circles[i].radius + circles[j].radius {
                    let tol_i = circles[i].plant_type.shade_tolerance();
                    let tol_j = circles[j].plant_type.shade_tolerance();
                    if tol_i < tol_j {
                        to_remove[i] = true;
                    } else if tol_j < tol_i {
                        to_remove[j] = true;
                    } else if rng::random_bool(0.5) {
                        to_remove[i] = true;
                    } else {
                        to_remove[j] = true;
                    }
                }
            }
        }
        let mut idx = 0;
        circles.retain(|_| {
            let keep = !to_remove[idx];
            idx += 1;
            keep
        });

        // Propagation: each plant may spawn one offspring nearby, inheriting its type
        let parents = circles.clone();
        for parent in &parents {
            if !rng::random_bool(0.25) {
                continue;
            }
            let angle = rng::random_range(0.0, TAU);
            let dist =
                rng::random_range(settings.kernel_radius * 0.5, settings.kernel_radius * 1.5);
            let new_pos = parent.pos + Vec2::new(angle.cos(), angle.sin()) * dist;
            if new_pos.x.abs() < half && new_pos.y.abs() < half {
                circles.push(PlantCircle {
                    pos: new_pos,
                    radius: 0.5,
                    plant_type: parent.plant_type,
                });
            }
        }
    }
}

fn circles_to_instances(
    circles: &[PlantCircle],
    settings: &EcosystemSettings,
) -> Vec<PlantInstance> {
    circles
        .iter()
        .map(|c| {
            let base_scale = match c.plant_type {
                PlantType::Tree => 25.0,
                PlantType::Bush => 15.0,
                _ => 10.0,
            };
            let max_r = settings.kernel_radius * 0.4;
            let scale = base_scale * (c.radius / max_r).clamp(0.5, 1.5);
            let rotation = rng::random_range(0.0, 360.0);
            let delay_years = rng::random_range(0.0, c.plant_type.years_per_iteration());
            PlantInstance::new(c.plant_type, delay_years).with_transform(
                vec3(c.pos.x, 0.0, c.pos.y),
                scale,
                rotation,
            )
        })
        .collect()
}

/// Generate a spatial distribution of plants according to the ecosystem settings.
///
/// Implements kernel-based placement (§4), optional self-thinning (§3.2),
/// and optional two-species succession (§3.3) from the multilevel plant
/// community paper.
pub fn generate_ecosystem_plants(settings: &EcosystemSettings, seed: u64) -> Vec<PlantInstance> {
    if settings.species.is_empty() {
        return Vec::new();
    }

    rng::seed(seed);

    // Build a normalised cumulative weight table for species selection
    let total_weight: f32 = settings.species.iter().map(|(_, w)| w).sum();
    let mut cum = 0f32;
    let cumulative: Vec<(PlantType, f32)> = settings
        .species
        .iter()
        .map(|(t, w)| {
            cum += w / total_weight;
            (*t, cum)
        })
        .collect();

    let sample_species = |r: f32| -> PlantType {
        for &(t, c) in &cumulative {
            if r <= c {
                return t;
            }
        }
        cumulative.last().unwrap().0
    };

    let mut grid = DensityGrid::new(settings.area);
    let mut circles: Vec<PlantCircle> = Vec::with_capacity(settings.num_plants as usize);

    for _ in 0..settings.num_plants {
        let pos = grid.sample();
        let plant_type = sample_species(rng::random_range(0.0, 1.0));
        grid.apply_kernel(pos.x, pos.y, settings.kernel, settings.kernel_radius);
        circles.push(PlantCircle {
            pos,
            radius: 1.0,
            plant_type,
        });
    }

    if settings.use_self_thinning {
        self_thinning(&mut circles, settings.thinning_radius);
    }

    if settings.use_succession {
        succession(&mut circles, settings);
    }

    circles_to_instances(&circles, settings)
}
