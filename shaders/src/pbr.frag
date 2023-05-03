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


layout(set = 2, binding = 0) uniform PointLight {
    vec3 position;
    vec3 color;
    float emissivity;
} point_light;

layout(location = 0) in VS_OUT {
    vec3 position;
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
    float occlusion = 1;
    float roughness = material_props.base_roughness;
    float metallic = material_props.base_metallic;
    vec3 albedo = material_props.base_color.rgb;
    vec3 normal = normalize(vs_out.tbn[2]);

    if (material_props.use_textures == 1) {
        occlusion *= texture(occlusion_roughness_metal_sampler, vs_out.tex_coord).r;
        roughness *= texture(occlusion_roughness_metal_sampler, vs_out.tex_coord).g;
        metallic *= texture(occlusion_roughness_metal_sampler, vs_out.tex_coord).b;
        albedo *= texture(base_color_sampler, vs_out.tex_coord).rgb;
        normal = texture(normal_sampler, vs_out.tex_coord).rgb;
        normal = normal * 2.0 - 1.0;
        normal = normalize(vs_out.tbn * normal);
    }

    vec3 view_direction = normalize(transforms.camera_position.xyz - vs_out.position);
    vec3 f0 = vec3(0.04);
    f0 = mix(f0, albedo, metallic);

    vec3 accumulated_lighting = vec3(0.0);

    // ------------------------ start per light calculations ------------------------
    vec3 light_direction = normalize(point_light.position - vs_out.position);
    float normal_dot_light = max(dot(normal, light_direction), 0.0);
    float normal_dot_view = max(dot(normal, view_direction), 0.0);
    vec3 half_vector = normalize(view_direction + light_direction);
    float light_distance = length(point_light.position - vs_out.position);
    float attenuation = point_light.emissivity / (light_distance * light_distance);
    vec3 radiance = point_light.color * attenuation;

    // cook-torrance brdf
    float normal_distribution_function = distribution_ggx(normal, half_vector, roughness);
    float geometry = geometry_smith(normal, view_direction, light_direction, roughness);
    vec3 fresnel = fresnel_schlick(max(dot(half_vector, view_direction), 0.0), f0);

    vec3 numerator = normal_distribution_function * geometry * fresnel;
    float denominator = 4.0 * normal_dot_view * normal_dot_light + 0.0001;
    vec3 specular = numerator / denominator;

    vec3 k_specular = fresnel;
    vec3 k_diffuse = vec3(1.0) - k_specular;
    k_diffuse *= 1.0 - metallic;

    accumulated_lighting += (k_diffuse * albedo / PI + specular) * radiance * normal_dot_light;

    // ------------------------ end per light calculations ------------------------

    vec3 ambient = vec3(0.03) * albedo * occlusion;
    vec3 color = ambient + accumulated_lighting;

    // reinhard tone map
    color = color / (color + vec3(1.0));

    // gamma correction done due by sRGB surface format
    out_color = vec4(color, 1.0);
}

float distribution_ggx(vec3 normal, vec3 half_vector, float roughness) {
    float a = roughness * roughness;
    float a2 = a * a;
    float NdotH = max(dot(normal, half_vector), 0.0);
    float NdotH2 = NdotH*NdotH;
    float nom = a2;
    float denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;
    return nom / denom;
}

float geometry_schlick_ggx(float normal_dot_view, float roughness)
{
    float r = (roughness + 1.0);
    float k = (r * r) / 8.0;
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
    return f0 + (1.0 - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}