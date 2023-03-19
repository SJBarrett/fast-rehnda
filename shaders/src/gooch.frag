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
    vec4 surface_color = texture(tex_sampler, frag_tex_coord) * material_props.base_color;
    vec4 cool_color = vec4(0.0, 0.0, 0.55, 1.0) * 0.1 + 0.9 * surface_color;
    vec4 warm_color = vec4(0.3, 0.3, 0.0, 1.0) * 0.1 + 0.9 * surface_color;
    vec3 light_dir = normalize(point_light.position - frag_position);
    vec3 normal = normalize(frag_normal);
    float t = (dot(light_dir, normal) + 1) / 2;
    vec4 kfinal = mix(cool_color, warm_color, t);

    float is_back = dot(normal, light_dir);
    float specular = 0.0;
    if (is_back > 0.0) {
        vec3 reflection = reflect(light_dir, normal);
        vec3 cam_direction = normalize(frag_position - transforms.camera_position.xyz);
        specular = pow(max(dot(reflection, cam_direction), 0.0), 32.0);
    }
    out_color = min(kfinal + specular, 1.0);
}
