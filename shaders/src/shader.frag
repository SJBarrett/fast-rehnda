#version 460

layout(set = 1, binding = 0) uniform sampler2D texSampler;
layout(set = 1, binding = 1) uniform MaterialProps {
    vec4 base_color;
} material_props;

layout(location = 0) in vec3 fragPosition;
layout(location = 1) in vec3 fragNormal;
layout(location = 2) in vec2 fragTexCoord;

layout(location = 0) out vec4 outColor;

struct Light {
    vec3 position;
    vec3 color;
};
const Light light = Light(vec3(5.0, 5.0, 5.0), vec3(1.0, 1.0, 1.0));
const vec3 camera_pos = vec3(1.5, -0.6, 9.7);

void main() {
    vec4 surface_color = texture(texSampler, fragTexCoord) * material_props.base_color;
    vec4 cool_color = vec4(0.0, 0.0, 0.55, 1.0) + 0.25 * surface_color;
    vec4 warm_color = vec4(0.3, 0.3, 0.0, 1.0) + 0.25 * surface_color;
    vec4 highlight = vec4(1.0, 1.0, 1.0, 1.0);
    vec3 light_dir = normalize(light.position - fragPosition);
    vec3 normal = normalize(fragNormal);
    float t = (dot(light_dir, normal) + 1) / 2;
    vec3 r = reflect(light_dir, normal);
    float s = clamp((100 * dot(r, normalize(fragPosition - camera_pos)) - 97), 0.0, 1.0);
    outColor = mix(mix(cool_color, warm_color, t), highlight, s);
}
