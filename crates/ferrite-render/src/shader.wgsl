struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec3<f32>,
};

// The texture and its sampler, bound from the CPU side as bind group 0.
@group(0) @binding(0) var t_sprite: texture_2d<f32>;
@group(0) @binding(1) var s_sprite: sampler;

// Vertex shader: passes the position through and forwards uv and tint color.
@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

// Fragment shader: reads the texture at the interpolated uv and applies the tint.
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(t_sprite, s_sprite, in.uv);
    return texel * vec4<f32>(in.color, 1.0);
}