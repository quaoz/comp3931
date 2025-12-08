struct Line {
    @location(0) start: vec3<f32>,
    @location(1) end: vec3<f32>,
    @location(2) colour: vec3<f32>,
}

struct Uniforms {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) colour: vec3<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    line: Line,
) -> VertexOutput {
    var out: VertexOutput;

    let position = select(line.end, line.start, vertex_index == 0u);
    out.clip_position = uniforms.view_proj * vec4<f32>(position, 1.0);
    out.colour = line.colour;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.colour / 255.0, 1.0);
}
