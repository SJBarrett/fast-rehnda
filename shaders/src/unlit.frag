#version 460

layout(set = 0, binding = 0) uniform TransformationMatrices {
    mat4 view;
    mat4 projection;
    vec4 camera_position;
} transforms;


layout(set = 1, binding = 0) uniform sampler2D base_color_sampler;
layout(set = 1, binding = 1) uniform sampler2D normal_sampler;
layout(set = 1, binding = 2) uniform sampler2D occlusion_roughness_metal_sampler;
layout(set = 1, binding = 3) uniform MaterialProps {
    vec4 base_color;
} material_props;

layout(location = 0) in vec3 frag_position;
layout(location = 1) in vec3 frag_normal;
layout(location = 2) in vec2 frag_tex_coord;
layout(location = 3) in mat3 tbn;

layout(location = 0) out vec4 out_color;

void main() {
    out_color = texture(base_color_sampler, frag_tex_coord) * material_props.base_color;
}
