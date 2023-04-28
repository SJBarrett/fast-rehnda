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

layout(location = 0) out vec3 frag_position;
layout(location = 1) out vec3 frag_normal;
layout(location = 2) out vec2 frag_tex_coord;
layout(location = 3) out mat3 tbn;

void main() {
    gl_Position = transforms.projection * transforms.view * constants.model * vec4(inPosition, 1.0);
    frag_tex_coord = inTexCoord;
    frag_normal = vec3(constants.normal_matrix * vec4(inNormal, 0));
    frag_position = (constants.model * vec4(inPosition, 1.0)).xyz;

    vec3 t = normalize(vec3(constants.model * vec4(inTangent.xyz, 0.0)));
    vec3 n = normalize(frag_normal);
    vec3 b = cross(n, t);
    tbn = mat3(t, b, n);
}