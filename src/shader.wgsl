struct VertexOutput {
    @builtin(position) position: vec4f,
};

// 顶点着色器-入口
@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out : VertexOutput;
    let x = f32(1- i32(in_vertex_index)) * 0.5;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
    out.position = vec4f(x, y, 0.0, 1.0);
    return out;
}

// 片元/片段着色器-入口
// 这里问了下AI，才知道片元/片段着色器是同一种东西的不同翻译
@fragment
fn fs_main(
    in: VertexOutput
) -> @location(0) vec4f {
    return vec4f(0.3, 0.2, 0.1, 1.0);
}