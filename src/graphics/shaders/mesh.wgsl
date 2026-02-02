struct Uniforms {
    view_proj: mat4x4<f32>,
    light_dir: vec3<f32>,
    ambient: f32,
    // season_tint multiplied against geometry colour each frame (no rebuild needed)
    season_tint: vec3<f32>,
    // implicit 4-byte padding to align to 16
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) colour: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
}

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) colour: vec3<f32>,
    @location(2) normal: vec3<f32>,
) -> VertexOutput {
    var out: VertexOutput;

    out.clip_position = uniforms.view_proj * vec4<f32>(position, 1.0);
    out.colour = colour;
    out.world_normal = normal;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let diffuse = max(dot(n, uniforms.light_dir), 0.0);
    let lighting = uniforms.ambient + (1.0 - uniforms.ambient) * diffuse;

    return vec4<f32>(in.colour * uniforms.season_tint * lighting, 1.0);
}
