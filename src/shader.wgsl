// 顶点着色器

struct Camera {
    // 文章中使用的是 mat4x4f，这里改为了 mat4x4<f32>
    // view_proj: mat4x4f
    view_proj: mat4x4<f32>,
}
@group(1) @binding(0)
var<uniform> camera: Camera;

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) tex_coords: vec2f,
}
struct InstanceInput {
    @location(5) model_matrix_0: vec4f,
    @location(6) model_matrix_1: vec4f,
    @location(7) model_matrix_2: vec4f,
    @location(8) model_matrix_3: vec4f,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) tex_coords: vec2f,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    // 第七章中：
    //这里将 model_matrix 从文章中的 mat4x4f 改为了 mat4x4<f32>，也运行正常，也不报语法错误了
    // let model_matrix = mat4x4f(
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = camera.view_proj * model_matrix * vec4f(model.position, 1.0);
    return out;
}

// 片元着色器

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0)@binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}