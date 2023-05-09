#version 460
layout(location = 0) in vec3 in_position;

layout(set = 0, binding = 0) uniform TransformationMatrices {
    mat4 view;
    mat4 projection;
    vec4 camera_position;
} transforms;

layout(location = 0) out vec3 out_position;

void main() {
    out_position = in_position;
    mat4 rot_view = mat4(mat3(transforms.view)); // remove translation from the view matrix
    vec4 clip_position = transforms.projection * rot_view * vec4(out_position, 1.0);
    // ensure the depth of the skybox is 1.0 so it's always rendered at the back
    gl_Position = clip_position.xyww;
}