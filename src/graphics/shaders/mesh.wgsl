struct Uniforms {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

const LIGHT_DIR: vec3<f32> = vec3<f32>(0.5773, 0.5773, -0.5773);
const AMBIENT: f32 = 0.3;

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
    out.colour = colour / 255.0;
    out.world_normal = normal;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let diffuse = max(dot(n, LIGHT_DIR), 0.0);
    let lighting = AMBIENT + (1.0 - AMBIENT) * diffuse;

    return vec4<f32>(in.colour * lighting, 1.0);
}
