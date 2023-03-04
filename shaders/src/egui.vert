#version 460

layout(location = 0) in vec2 inPosition;
layout(location = 1) in vec2 inTexCoord;
layout(location = 2) in uint inColor;

layout(push_constant) uniform PushConstants {
    vec2 screen_size;
} constants;

layout(location = 0) out vec2 fragTexCoord;
layout(location = 1) out vec4 fragColor;

vec3 srgb_to_linear(vec3 srgb)
{
    bvec3 cutoff = lessThan(srgb, vec3(0.040449999272823333740234375));
    vec3 lower = srgb / vec3(12.9200000762939453125);
    vec3 higher = pow((srgb + vec3(0.054999999701976776123046875)) / vec3(1.05499994754791259765625), vec3(2.400000095367431640625));
    return mix(higher, lower, cutoff);
}

vec4 unpack_color(uint color) {
    return vec4(
        color & 255u,
        (color >> 8u) & 255u,
        (color >> 16u) & 255u,
        (color >> 24u) & 255u
    );
}

vec4 position_from_screen(vec2 screen_position) {
    return vec4(
        2 * screen_position.x / constants.screen_size.x - 1,
        2 * screen_position.y / constants.screen_size.y - 1,
        0.0,
        1.0
    );
}

void main() {
    gl_Position = position_from_screen(inPosition);
    fragTexCoord = inTexCoord;
    vec4 unpacked_color = unpack_color(inColor);
    fragColor = vec4(srgb_to_linear(unpacked_color.rgb), unpacked_color.a);
}

