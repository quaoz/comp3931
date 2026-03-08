struct Uniforms {
    view_proj: mat4x4<f32>, // 0–63
    light_dir: vec3<f32>,   // 64–75
    ambient: f32,         // 76–79
    season_tint: vec3<f32>,   // 80–91  (multiplied per-frame, no scene rebuild needed)
    time: f32,         // 92–95  (elapsed seconds, wraps at ~6h)
    wind_anim_strength: f32,         // 96–99  (0 = off / screenshot mode)
    wind_dir_x: f32,         // 100–103
    wind_dir_z: f32,         // 104–107
    _pad: f32,         // 108–111 (struct size = 112, align = 16)
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0) var t_leaf_colour: texture_2d<f32>;
@group(1) @binding(1) var t_leaf_opacity: texture_2d<f32>;
@group(1) @binding(2) var t_leaf_normal: texture_2d<f32>;
@group(1) @binding(3) var s_leaf: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) colour: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) world_tangent: vec3<f32>,
}

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) colour: vec3<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) tangent: vec3<f32>,
) -> VertexOutput {
    // Wind animation: h²-weighted sway anchors the base while tips move maximally.
    // Two-frequency sine waves (primary slow + secondary fast) produce organic motion.
    // When wind_anim_strength == 0.0 (screenshot mode), sway == 0 and position is unchanged.
    let h = max(position.y, 0.0);
    let h2 = h * h;
    let phase1 = uniforms.time * 0.7 + position.x * 0.13 + position.z * 0.11;
    let phase2 = uniforms.time * 2.3 + position.x * 0.29 + position.z * 0.31;
    let sway = (h2 * sin(phase1) + h * 0.25 * sin(phase2)) * uniforms.wind_anim_strength;
    let displaced = vec3<f32>(
        position.x + uniforms.wind_dir_x * sway,
        position.y,
        position.z + uniforms.wind_dir_z * sway,
    );

    var out: VertexOutput;
    out.clip_position = uniforms.view_proj * vec4<f32>(displaced, 1.0);
    out.colour = colour;
    out.world_normal = normal;
    out.uv = uv;
    out.world_tangent = tangent;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let is_leaf = in.uv.x >= 0.0;

    // Opacity discard for leaves
    if is_leaf {
        let alpha = textureSample(t_leaf_opacity, s_leaf, in.uv).r;
        if alpha < 0.1 {
            discard;
        }
    }

    // Normal — use normal map for leaves, face normal for branches
    var n: vec3<f32>;
    if is_leaf {
        let N = normalize(in.world_normal);
        // Gram-Schmidt re-orthogonalise tangent against normal
        let T = normalize(in.world_tangent - dot(in.world_tangent, N) * N);
        let B = cross(N, T);
        // Decode tangent-space normal: [0,1]^3 → [-1,1]^3
        let ts_n = textureSample(t_leaf_normal, s_leaf, in.uv).rgb * 2.0 - 1.0;
        n = normalize(T * ts_n.x + B * ts_n.y + N * ts_n.z);
    } else {
        n = normalize(in.world_normal);
    }

    // Lambert lighting
    let diffuse = max(dot(n, uniforms.light_dir), 0.0);
    let lighting = uniforms.ambient + (1.0 - uniforms.ambient) * diffuse;

    // Base colour: texture × vertex colour tint for leaves, vertex colour for branches
    var base_colour: vec3<f32>;
    if is_leaf {
        let tex_colour = textureSample(t_leaf_colour, s_leaf, in.uv).rgb;
        base_colour = tex_colour * in.colour;
    } else {
        base_colour = in.colour;
    }

    return vec4<f32>(base_colour * uniforms.season_tint * lighting, 1.0);
}
