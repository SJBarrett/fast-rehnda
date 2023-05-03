#version 460

layout(set = 0, binding = 0) uniform TransformationMatrices {
    mat4 view;
    mat4 projection;
    vec4 camera_position;
} transforms;


layout(set = 1, binding = 0) uniform MaterialProps {
    vec4 base_color;
    float base_roughness;
    float base_metallic;
    int use_textures;
} material_props;
layout(set = 1, binding = 1) uniform sampler2D base_color_sampler;
layout(set = 1, binding = 2) uniform sampler2D normal_sampler;
layout(set = 1, binding = 3) uniform sampler2D occlusion_roughness_metal_sampler;

layout(location = 0) in VS_OUT {
    vec3 position;
    vec2 tex_coord;
    mat3 tbn;
} vs_out;

layout(location = 0) out vec4 out_color;

void main() {
    out_color = texture(base_color_sampler, vs_out.tex_coord) * material_props.base_color;
}
