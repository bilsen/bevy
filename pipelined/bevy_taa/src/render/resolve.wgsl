


[[group(0), binding(0)]]
var color: texture_2d<f32>;
[[group(0), binding(1)]]
var depth: texture_depth_2d;
[[group(0), binding(2)]]
var velocity: texture_2d;
[[group(0), binding(3)]]
var previous_color: texture_2d<f32>;
[[group(0), binding(4)]]
var previous_depth: texture_depth_2d;
[[group(0), binding(5)]]
var sampler: sampler;




struct VertexOutput {
	[[builtin(position)]] position: vec4<f32>;
	[[location(0)]] uv: vec2<f32>;
};

var vertices: array<vec2<f32>, 3> = array<vec2<f32>, 3>(
	vec2<f32>(-1.0, -1.0),
	vec2<f32>(3.0, -1.0),
	vec2<f32>(-1.0, 3.0),
);

// full screen triangle vertex shader
[[stage(vertex)]]
fn vertex([[builtin(vertex_index)]] idx: u32) -> VertexOutput {
	var out: VertexOutput;

	out.position = vec4<f32>(vertices[idx], 0.0, 1.0);
	out.uv = vertices[idx] * vec2<f32>(0.5, -0.5);
	out.uv = out.uv + 0.5;

	return out;
}


[[stage(fragment)]]
fn fragment(in: VertexOutput) -> [[location(0)]] vec3<f32> {

	return vec3<f32>(0.0, 0.0, 0.0);
}

