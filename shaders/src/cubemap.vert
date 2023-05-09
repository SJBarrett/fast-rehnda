#version 460
layout(location = 0) in vec3 in_position;

layout(location = 0) out vec3 out_position;

layout(push_constant) uniform PushConstants {
    mat4 projection;
    mat4 view;
} constants;

void main() {
    out_position = in_position;
    gl_Position = constants.projection * constants.view * vec4(out_position, 1.0);
}