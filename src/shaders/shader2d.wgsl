// This shader defines a simple 2D triangle
// Wayne Materi - Jan 2025
// based on WGPU for beginners 3: Shaders on YouTube
// rev. 2025-02-06 9:37 a.m. 

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct Transform {
    position: vec4<f32>,
    scale: f32,
    color: vec4<f32>,
};


@group(0) @binding(0) var<storage, read> transforms: array<Transform>;

struct VertexOutput {
    @builtin(position) posn: vec4<f32>,
    @location(0) color: vec4<f32>,
};


@vertex
fn vs_main(@builtin(instance_index) instance_idx: u32, model: VertexInput) -> VertexOutput {
    let transform = transforms[instance_idx];
    var output: VertexOutput;
    //output.posn = transform * vec4<f32>(model.position, 1.0);
    output.posn = vec4<f32>(model.position, 1.0);
    output.color = transform.color;
    return output;
}

@fragment
fn fs_main(@location(0) color: vec4<f32>) -> @location(0) vec4<f32> {
    return color;
}

/*@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = transform * vec4<f32>(model.position, 1.0); // Apply transformation
    out.color = model.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
*/