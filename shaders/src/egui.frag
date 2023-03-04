#version 460

layout(location = 0) in vec2 fragTexCoord;
layout(location = 1) in vec4 fragColor;

layout(set = 0, binding = 0) uniform sampler2D texSampler;

layout(location = 0) out vec4 outColor;

vec3 srgb_to_linear(vec3 srgb)
{
    bvec3 cutoff = lessThan(srgb, vec3(0.040449999272823333740234375));
    vec3 lower = srgb / vec3(12.9200000762939453125);
    vec3 higher = pow((srgb + vec3(0.054999999701976776123046875)) / vec3(1.05499994754791259765625), vec3(2.400000095367431640625));
    return mix(higher, lower, cutoff);
}

void main() {
    vec4 tex_linear = texture(texSampler, fragTexCoord);
    outColor = fragColor * vec4(srgb_to_linear(tex_linear.rgb), tex_linear.a);
}
