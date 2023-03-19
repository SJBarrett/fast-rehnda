#version 460

layout(location = 0) in vec2 frag_tex_coord;
layout(location = 1) in vec4 fragColor;

layout(set = 0, binding = 0) uniform sampler2D tex_sampler;

layout(location = 0) out vec4 out_color;

void main() {
    vec4 tex_linear = texture(tex_sampler, frag_tex_coord);
    out_color = fragColor * tex_linear;
}
