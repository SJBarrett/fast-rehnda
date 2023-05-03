#version 460

layout(set = 0, binding = 0) uniform TransformationMatrices {
    mat4 view;
    mat4 projection;
    vec4 camera_position;
} transforms;

layout(push_constant) uniform PushConstants {
    mat4 model;
    mat4 normal_matrix;
} constants;

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inTexCoord;
layout(location = 3) in vec4 inTangent;

layout(location = 0) out VS_OUT {
    vec3 position;
    vec2 tex_coord;
    mat3 tbn;
} vs_out;

void main() {
    gl_Position = transforms.projection * transforms.view * constants.model * vec4(inPosition, 1.0);
    vs_out.tex_coord = inTexCoord;
    vec3 normal = vec3(constants.normal_matrix * vec4(inNormal, 0));
    vs_out.position = (constants.model * vec4(inPosition, 1.0)).xyz;

    vec3 t = normalize(vec3(constants.model * vec4(inTangent.xyz, 0.0)));
    vec3 n = normalize(normal);
    t = normalize(t - dot(t, n) * n);
    vec3 b = cross(n, t);
    vs_out.tbn = mat3(t, b, n);
}