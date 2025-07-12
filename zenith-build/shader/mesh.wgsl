struct ViewUniforms {
    view_proj: mat4x4<f32>,
}

struct ModelUniforms {
    model: mat4x4<f32>,
    base_color: vec3<f32>,
}

@group(0) @binding(0)
var<uniform> view: ViewUniforms;

@group(0) @binding(1)
var<uniform> model: ModelUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    let world_pos = model.model * vec4<f32>(input.position, 1.0);
    output.position = view.view_proj * world_pos;

    output.world_normal = (model.model * vec4<f32>(input.normal, 0.0)).xyz;
    output.tex_coord = input.tex_coord;

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.world_normal);
    let light_intensity = max(dot(normal, vec3<f32>(0.0, 0.0, 1.0)), 0.3);

    let final_color = model.base_color * light_intensity;
    return vec4<f32>(final_color, 1.0);
} 