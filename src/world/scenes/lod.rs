use glam::{Mat4, Vec3, Vec4};

use crate::{
    settings::{CullSettings, LodSettings},
    util::turtle::{LineGeometry, MeshGeometry},
};

// ── LOD tier enum ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LodTier {
    /// Full detail: 8 cylinder segments, all branches, all leaves
    Near = 0,
    /// Medium detail: 5 cylinder segments, reduced leaf density
    Mid = 1,
    /// Low detail: 3 cylinder segments, thin branches culled, reduced leaf density
    Far = 2,
    /// Beyond far threshold: no mesh, line skeleton only, no leaves
    Beyond = 3,
    /// Outside view frustum: nothing rendered
    Culled = 4,
}

impl LodTier {
    /// Returns `(cylinder_segments, min_branch_radius)` for geometry generation.
    pub fn geometry_params(self) -> (usize, f32) {
        match self {
            LodTier::Near => (8, 0.0),
            LodTier::Mid => (5, 0.0),
            LodTier::Far => (3, 0.005),
            _ => (0, 1000.0),
        }
    }

    /// Returns the leaf-quad skip probability for this tier from settings.
    /// `Beyond` and `Culled` always skip all leaves.
    pub fn leaf_skip(self, lod: &LodSettings) -> f32 {
        match self {
            LodTier::Near => lod.leaf_skip_near,
            LodTier::Mid => lod.leaf_skip_mid,
            LodTier::Far => lod.leaf_skip_far,
            _ => 1.0,
        }
    }
}

pub fn lod_tier(dist: f32, lod: &LodSettings) -> LodTier {
    if dist < lod.near_threshold {
        LodTier::Near
    } else if dist < lod.mid_threshold {
        LodTier::Mid
    } else if dist < lod.far_threshold {
        LodTier::Far
    } else {
        LodTier::Beyond
    }
}

/// Like `lod_tier` but applies a hysteresis dead zone at each boundary so that a
/// plant oscillating near a threshold does not repeatedly trigger rebuilds.
pub fn lod_tier_smooth(
    dist: f32,
    prev_tier: LodTier,
    lod: &LodSettings,
    hysteresis: f32,
) -> LodTier {
    let raw = lod_tier(dist, lod);
    if raw == prev_tier || prev_tier == LodTier::Culled {
        return raw;
    }

    let boundary = match raw.min(prev_tier) {
        LodTier::Near => lod.near_threshold,
        LodTier::Mid => lod.mid_threshold,
        LodTier::Far => lod.far_threshold,
        _ => return raw,
    };

    if raw > prev_tier {
        if dist > boundary + hysteresis {
            raw
        } else {
            prev_tier
        }
    } else {
        if dist < boundary - hysteresis {
            raw
        } else {
            prev_tier
        }
    }
}

// ── Frustum culling ──

/// Six half-space planes extracted from a view-projection matrix
pub struct Frustum {
    planes: [Vec4; 6],
}

impl Frustum {
    pub fn from_view_proj(vp: Mat4) -> Self {
        let t = vp.transpose();
        let planes_raw = [
            t.w_axis + t.x_axis, // left
            t.w_axis - t.x_axis, // right
            t.w_axis + t.y_axis, // bottom
            t.w_axis - t.y_axis, // top
            t.w_axis + t.z_axis, // near
            t.w_axis - t.z_axis, // far
        ];
        let planes = planes_raw.map(|p| {
            let len = p.truncate().length();
            if len > 1e-6 { p / len } else { p }
        });
        Self { planes }
    }

    /// Returns `false` only when the sphere is fully outside at least one frustum plane
    pub fn contains_sphere(&self, centre: Vec3, radius: f32) -> bool {
        self.planes
            .iter()
            .all(|&plane| plane.dot(centre.extend(1.0)) >= -radius)
    }
}

/// Returns the LOD tier for a plant, applying frustum culling and hysteresis
pub fn plant_lod_tier(
    pos: Vec3,
    camera_pos: Vec3,
    prev_tier: LodTier,
    frustum: &Frustum,
    lod: &LodSettings,
    cull: &CullSettings,
    full_rebuild: bool,
) -> LodTier {
    if cull.frustum_culling && !frustum.contains_sphere(pos, cull.cull_radius) {
        return LodTier::Culled;
    }

    let dist = (pos - camera_pos).length();
    if full_rebuild {
        lod_tier(dist, lod)
    } else {
        lod_tier_smooth(dist, prev_tier, lod, cull.lod_hysteresis)
    }
}

// ── Per-plant geometry cache ──

pub struct PlantCache {
    /// Tier used for geometry
    pub tier: LodTier,
    /// Tier requested by distance/frustum before any adjustment
    pub requested_tier: LodTier,
    pub line_geo: LineGeometry,
    pub mesh_geo: MeshGeometry,
}

impl PlantCache {
    pub fn empty(tier: LodTier) -> Self {
        Self {
            tier,
            requested_tier: tier,
            line_geo: LineGeometry {
                vertices: vec![],
                colours: vec![],
                indices: vec![],
                segments: vec![],
            },
            mesh_geo: MeshGeometry {
                vertices: vec![],
                normals: vec![],
                colours: vec![],
                indices: vec![],
                uvs: vec![],
                tangents: vec![],
            },
        }
    }
}

// ── Scene Stats ──

/// Snapshot of per-frame scene statistics for the debug HUD.
#[derive(Clone, Copy)]
pub struct SceneStats {
    pub plant_count: usize,
    pub culled_count: usize,
    /// Plants at each LOD tier (index = LodTier discriminant, 0–3 for Near..Beyond).
    pub lod_tier_counts: [usize; 4],
    /// Milliseconds taken by the most recent scene build (0 if served from cache).
    pub last_rebuild_ms: f32,
    /// Whether the last rebuild was a full rebuild or a partial (tier-only) update.
    pub last_rebuild_full: bool,
    pub line_vertex_count: u32,
    /// Active LOD pressure multiplier (1.0 = normal; < 1.0 = geometry budget exceeded).
    pub lod_pressure: f32,
    /// Effective LOD distance thresholds after pressure scaling: [near, mid, far].
    pub lod_thresholds: [f32; 3],
}

impl Default for SceneStats {
    fn default() -> Self {
        Self {
            plant_count: 0,
            culled_count: 0,
            lod_tier_counts: [0; 4],
            last_rebuild_ms: 0.0,
            last_rebuild_full: false,
            line_vertex_count: 0,
            lod_pressure: 1.0,
            lod_thresholds: [80.0, 160.0, 300.0],
        }
    }
}
