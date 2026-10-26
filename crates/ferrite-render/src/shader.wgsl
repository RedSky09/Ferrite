// Vertex shader: places the three corners of the triangle.
// Positions are in clip space: x and y range from -1 to 1, (0, 0) is the center.
@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>( 0.0,  0.5),
        vec2<f32>(-0.5, -0.5),
        vec2<f32>( 0.5, -0.5),
    );
    return vec4<f32>(pos[i], 0.0, 1.0);
}

// Fragment shader: picks the color of every pixel inside the triangle.
@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.5, 0.2, 1.0);
}