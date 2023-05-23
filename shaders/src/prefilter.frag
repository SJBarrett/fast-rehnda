#version 460
layout(location = 0) in vec3 in_position;

layout(location = 0) out vec4 out_color;
layout(set = 0, binding = 0) uniform samplerCube environment_map;
layout(set = 1, binding = 0) uniform PrefilterProps {
    float roughness;
} props;

const float PI = 3.14159265359;

float radical_inverse_vdc(uint bits);
vec2 hammersley(uint i, uint N);
vec3 importance_sample_ggx(vec2 xi, vec3 normal, float roughness);

// based off https://learnopengl.com/PBR/IBL/Specular-IBL
void main() {
    // IMPROVEMENT stop requiring this change needing to be done in the shader
    // flip the y
    vec3 normal = in_position;
    normal.y = -normal.y;
    normal = normalize(normal);
    vec3 r = normal;
    vec3 v = r;

    const uint SAMPLE_COUNT = 1024u;
    float total_weight = 0.0;
    vec3 prefiltered_color = vec3(0.0);
    for (uint i = 0u; i < SAMPLE_COUNT; ++i) {
        vec2 xi = hammersley(i, SAMPLE_COUNT);
        vec3 h = importance_sample_ggx(xi, normal, props.roughness);
        vec3 l = normalize(2.0 * dot(v, h) * h - v);

        float normal_dot_l = max(dot(normal, l), 0.0);
        if (normal_dot_l > 0.0) {
            prefiltered_color += texture(environment_map, l).rgb * normal_dot_l;
            total_weight += normal_dot_l;
        }
    }
    prefiltered_color = prefiltered_color / total_weight;
    out_color = vec4(prefiltered_color, 1.0);
}

// generate a Van Der Corput sequence
float radical_inverse_vdc(uint bits) {
    bits = (bits << 16u) | (bits >> 16u);
    bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
    bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
    bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
    bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
    return float(bits) * 2.3283064365386963e-10;
}

vec2 hammersley(uint i, uint n) {
    return vec2(float(i) / float(n), radical_inverse_vdc(i));
}

vec3 importance_sample_ggx(vec2 xi, vec3 normal, float roughness) {
    float a = roughness * roughness;

    float phi = 2.0 * PI * xi.x;
    float cos_theta = sqrt((1.0 - xi.y) / (1.0 + (a * a - 1.0) * xi.y));
    float sin_theta = sqrt(1.0 - cos_theta * cos_theta);

    // from spherical coordinates to cartesian coordinates
    vec3 h;
    h.x = cos(phi) * sin_theta;
    h.y = sin(phi) * sin_theta;
    h.z = cos_theta;

    // from tangent-space vector to world-space sample vector
    vec3 up        = abs(normal.z) < 0.999 ? vec3(0.0, 0.0, 1.0) : vec3(1.0, 0.0, 0.0);
    vec3 tangent   = normalize(cross(up, normal));
    vec3 bitangent = cross(normal, tangent);

    vec3 sample_vec = tangent * h.x + bitangent * h.y + normal * h.z;
    return normalize(sample_vec);
}