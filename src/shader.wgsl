// 第四章 缓冲区与索引
struct VertexInput {
    @location(0) position: vec3f,
    @location(1) color: vec3f,
}

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) color: vec3f,
};

// 顶点着色器-入口
@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out : VertexOutput;
    out.color = model.color;
    out.position = vec4f(model.position, 1.0);
    return out;
}

// 片元/片段着色器-入口
// 这里问了下AI，才知道片元/片段着色器是同一种东西的不同翻译
@fragment
fn fs_main(
    in: VertexOutput
) -> @location(0) vec4f {
    return vec4f(in.color, 1.0);
}