#version 460

layout(location = 0) in vec2 fragTexCoord;
layout(location = 1) in vec4 fragColor;

layout(set = 0, binding = 0) uniform sampler2D texSampler;

layout(location = 0) out vec4 outColor;

void main() {
    vec4 tex_linear = texture(texSampler, fragTexCoord);
    outColor = fragColor * tex_linear;
}
