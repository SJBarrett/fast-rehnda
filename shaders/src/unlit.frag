#version 460

layout(set = 0, binding = 0) uniform TransformationMatrices {
    mat4 view;
    mat4 projection;
    vec4 camera_position;
} transforms;

layout(set = 1, binding = 0) uniform sampler2D tex_sampler;
layout(set = 1, binding = 1) uniform MaterialProps {
    vec4 base_color;
} material_props;

layout(location = 0) in vec3 frag_position;
layout(location = 1) in vec3 frag_normal;
layout(location = 2) in vec2 frag_tex_coord;

layout(location = 0) out vec4 out_color;

void main() {
    out_color = texture(tex_sampler, frag_tex_coord) * material_props.base_color;
}
