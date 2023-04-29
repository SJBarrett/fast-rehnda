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

float distribution_ggx(vec3 normal, vec3 half_vector, float a);
float geometry_schlick_ggx(float normal_dot_view, float k);
float geometry_smith(vec3 normal, vec3 view_direction, vec3 light_direction, float k);
vec3 fresnel_schlick(float cos_theta, vec3 f0);

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

float distribution_ggx(vec3 normal, vec3 half_vector, float a) {
    float a2 = a*a;
    float NdotH = max(dot(normal, half_vector), 0.0);
    float NdotH2 = NdotH*NdotH;
    float nom = a2;
    float denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;
    return nom / denom;
}

float geometry_schlick_ggx(float normal_dot_view, float k)
{
    float nom   = normal_dot_view;
    float denom = normal_dot_view * (1.0 - k) + k;

    return nom / denom;
}

float geometry_smith(vec3 normal, vec3 view_direction, vec3 light_direction, float k)
{
    float normal_dot_view = max(dot(normal, view_direction), 0.0);
    float normal_dot_light = max(dot(normal, light_direction), 0.0);
    float ggx1 = geometry_schlick_ggx(normal_dot_view, k);
    float ggx2 = geometry_schlick_ggx(normal_dot_light, k);

    return ggx1 * ggx2;
}

vec3 fresnel_schlick(float cos_theta, vec3 f0)
{
    return f0 + (1.0 - f0) * pow(1.0 - cos_theta, 5.0);
}