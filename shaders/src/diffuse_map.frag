#version 460
layout(location = 0) in vec3 in_position;

layout(location = 0) out vec4 out_color;
layout(set = 0, binding = 0) uniform samplerCube environment_map;

const float PI = 3.14159265359;

void main() {
    vec3 normal = in_position;
    normal.y = -normal.y;
    normal = normalize(normal);
    vec3 irradiance = vec3(0.0);

    // sourced from https://learnopengl.com/PBR/IBL/Diffuse-irradiance
    vec3 up = vec3(0.0, 1.0, 0.0);
    vec3 right = normalize(cross(up, normal));
    up = normalize(cross(normal, right));

    float sample_delta = 0.025;
    int samples_taken = 0;
    // go around the hemisphere
    for (float phi = 0.0; phi < 2.0 * PI; phi += sample_delta) {
        // go up an arc
        for (float theta = 0.25 * PI; theta < 0.5 * PI; theta += sample_delta) {
            // spherical to cartesian (in tangent space)
            vec3 tangent_sample = vec3(sin(theta) * cos(phi), sin(theta) * sin(phi), cos(theta));
            // tangent space to world
            vec3 sample_vector = tangent_sample.x * right + tangent_sample.y * up + tangent_sample.z * normal;

            // scale by cos(theta) as light at grazing angles has less power, scale by sin(theta) due to increased
            // sampling at the top of the hemisphere where points converge
            irradiance += texture(environment_map, sample_vector).rgb * cos(theta) * sin(theta);
            samples_taken++;
        }
    }
    irradiance = PI * irradiance * (1.0 / float(samples_taken));
    out_color = vec4(irradiance, 1.0);
}