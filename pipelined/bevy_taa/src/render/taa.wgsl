

[[block]]
struct View {
    view_proj: mat4x4<f32>;
    projection: mat4x4<f32>;
    world_position: vec3<f32>;
};

[[block]]
struct Mesh {
    model: mat4x4<f32>;
    inverse_transpose_model: mat4x4<f32>;
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    flags: u32;
};


[[group(0), binding(0)]]
var view: View;
[[group(0), binding(1)]]
var previous_view: View;

[[group(1), binding(0)]]
var mesh: Mesh;
[[group(1), binding(1)]]
var previous_mesh: Mesh;


struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] current_pos: vec4<f32>;
    [[location(1)]] previous_pos: vec4<f32>;
};


[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    let world_position = mesh.model * vec4<f32>(vertex.position, 1.0);

    var o: VertexOutput;
    o.clip_position = view.view_proj * world_position;

    o.current_pos = mesh.model * vec4<f32>(vertex.position, 1.0);
    o.current_pos = view.view_proj * o.current_pos;

    o.previous_pos = previous_mesh.model * vec4<f32>(vertex.position, 1.0);
    o.previous_pos = previous_view.view_proj * o.previous_pos;


    return o;
}


struct FragmentInput {
    [[location(0)]] current_pos: vec4<f32>;
    [[location(1)]] previous_pos: vec4<f32>;
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec2<f32> {
    
    var current_pos_NDC: vec3<f32> = in.current_pos.xyz / in.current_pos.w;
    var previous_pos_NDC: vec3<f32> = in.previous_pos.xyz / in.previous_pos.w;
    var velocity: vec2<f32> = current_pos_NDC.xy - previous_pos_NDC.xy;
    // return vec2<f32>(0.5, 0.5);
    return current_pos_NDC.xy - previous_pos_NDC.xy;
}
