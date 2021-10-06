


[[group(0), binding(0)]]
var color: texture_2d<f32>;
[[group(0), binding(1)]]
var color_sampler: sampler;
[[group(0), binding(2)]]
var depth: texture_depth_2d;
[[group(0), binding(3)]]
var depth_sampler: sampler;
[[group(0), binding(4)]]
var velocity: texture_2d;
[[group(0), binding(5)]]
var velocity_sampler: sampler;


[[group(1), binding(0)]]
var previous_color: texture_2d<f32>;
[[group(1), binding(1)]]
var previous_color_sampler: sampler;
[[group(1), binding(2)]]
var previous_depth: texture_depth_2d;
[[group(1), binding(3)]]
var previous_depth_sampler: sampler;


