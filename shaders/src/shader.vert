#version 460

layout(set = 0, binding = 0) uniform TransformationMatrices {
    mat4 view;
    mat4 projection;
    vec4 camera_position;
} transforms;

layout(push_constant) uniform PushConstants {
    mat4 model;
} constants;

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inTexCoord;

layout(location = 0) out vec3 frag_position;
layout(location = 1) out vec3 frag_normal;
layout(location = 2) out vec2 frag_tex_coord;

void main() {
    gl_Position = transforms.projection * transforms.view * constants.model * vec4(inPosition, 1.0);
    frag_tex_coord = inTexCoord;
    frag_normal = inNormal;
    frag_position = (constants.model * vec4(inPosition, 1.0)).xyz;
}