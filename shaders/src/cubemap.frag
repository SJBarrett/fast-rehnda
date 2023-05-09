#version 460
layout(location = 0) in vec3 in_position;

layout(location = 0) out vec4 out_color;
layout(set = 0, binding = 0) uniform sampler2D equirectanguler_map;

const vec2 inverse_a_tan = vec2(0.1591, 0.3183);
vec2 sample_spherical_map(vec3 v) {
    vec2 uv = vec2(atan(v.z, v.x), asin(v.y));
    uv *= inverse_a_tan;
    uv += 0.5;
    return uv;
}

void main() {
    vec2 uv = sample_spherical_map(normalize(in_position));
    vec3 color = texture(equirectanguler_map, uv).rgb;
    out_color = vec4(color, 1.0);
}