// 第四章 缓冲区与索引
struct VertexInput {
    @location(0) position: vec3f,
    @location(1) tex_coords: vec2f,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) tex_coords: vec2f,
};

// 顶点着色器-入口
@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out : VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = vec4f(model.position, 1.0);
    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

// 片元/片段着色器-入口
// 这里问了下AI，才知道片元/片段着色器是同一种东西的不同翻译
@fragment
fn fs_main(
    in: VertexOutput
) -> @location(0) vec4f {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}