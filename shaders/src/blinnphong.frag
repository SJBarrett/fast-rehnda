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

layout(location = 0) in VS_OUT {
    vec3 position;
    vec3 normal;
    vec2 tex_coord;
    mat3 tbn;
} vs_out;


layout(location = 0) out vec4 out_color;

const float PI = 3.14159265359;

void main() {
    float occlusion = texture(occlusion_roughness_metal_sampler, vs_out.tex_coord).r;
    float roughness = texture(occlusion_roughness_metal_sampler, vs_out.tex_coord).g;
    float metallic = texture(occlusion_roughness_metal_sampler, vs_out.tex_coord).b;
    vec3 normal = texture(normal_sampler, vs_out.tex_coord).rgb;
    normal = normal * 2.0 - 1.0;
    normal = normalize(vs_out.tbn * normal);


    // linearise the sRGB texture
    vec3 albedo = texture(base_color_sampler, vs_out.tex_coord).rgb;

    // ambient lighting
    float ambient_strength = 0.1;
    vec3 ambient = ambient_strength * point_light.color;

    // diffuse lighting
    vec3 light_direction = normalize(point_light.position - vs_out.position);
    float diff = max(dot(normal, light_direction), 0.0);
    vec3 diffuse = diff * point_light.color;

    // specular (Blinn-Phong specular)
    float specular_strength = 0.5;
    vec3 view_direction = normalize(transforms.camera_position.xyz - vs_out.position);
    float incidence_angle = clamp(dot(normal, light_direction), 0, 1);

    vec3 halfway_direction = normalize(light_direction + view_direction);
    float blinn_term = dot(normal, halfway_direction);
    blinn_term = clamp(blinn_term, 0, 1);
    blinn_term = incidence_angle != 0.0 ? blinn_term : 0.0;
    blinn_term = pow(blinn_term, 256);
    vec3 specular = specular_strength * blinn_term * point_light.color;


    vec3 result = (ambient + diffuse + specular) * albedo;

    out_color = vec4(result, 1.0);
}
