#version 460

layout(location = 0) in vec2 inPosition;
layout(location = 1) in vec2 inTexCoord;
layout(location = 2) in uint inColor;

layout(push_constant) uniform PushConstants {
    vec2 screen_size;
} constants;

layout(location = 0) out vec2 fragTexCoord;
layout(location = 1) out vec4 fragColor;

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
    fragColor = unpack_color(inColor);
}

