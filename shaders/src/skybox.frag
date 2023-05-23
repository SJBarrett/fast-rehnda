#version 460
layout(location = 0) in vec3 in_position;

layout(location = 0) out vec4 out_color;

layout(set = 1, binding = 0) uniform samplerCube cube_map;

void main() {
    vec3 color = texture(cube_map, in_position).rgb;
    color = color / (color + vec3(1.0));
    out_color = vec4(color, 1.0);
}