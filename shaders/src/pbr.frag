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

layout(set = 2, binding = 0) uniform PointLight {
    vec3 position;
    vec3 color;
    float emissivity;
} point_light;

layout(location = 0) in vec3 frag_position;
layout(location = 1) in vec3 frag_normal;
layout(location = 2) in vec2 frag_tex_coord;

layout(location = 0) out vec4 out_color;

void main() {
    float occlusion = texture(occlusion_roughness_metal_sampler, frag_tex_coord).r;
    float roughness = texture(occlusion_roughness_metal_sampler, frag_tex_coord).g;
    float metal = texture(occlusion_roughness_metal_sampler, frag_tex_coord).b;
    vec4 normal = texture(normal_sampler, frag_tex_coord);
    vec4 base_color = texture(base_color_sampler, frag_tex_coord);
    out_color = vec4(roughness, roughness, roughness, 1.0);
}

