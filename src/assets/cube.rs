use std::mem::size_of;
use ash::vk;
use crate::rehnda_core::Vec3;

pub fn cube_vertex_attributes() -> Vec<vk::VertexInputAttributeDescription> {
    vec![
        vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build()
    ]
}

pub fn cube_vertex_input_bindings() -> vk::VertexInputBindingDescription {
    vk::VertexInputBindingDescription::builder()
        .binding(0)
        .stride(size_of::<Vec3>() as u32)
        .input_rate(vk::VertexInputRate::VERTEX)
        .build()
}

pub const CUBE_VERTICES: [f32; 108] = [
    // positions          
    -1.0f32,  1.0f32, -1.0f32,
    -1.0f32, -1.0f32, -1.0f32,
    1.0f32, -1.0f32, -1.0f32,
    1.0f32, -1.0f32, -1.0f32,
    1.0f32,  1.0f32, -1.0f32,
    -1.0f32,  1.0f32, -1.0f32,

    -1.0f32, -1.0f32,  1.0f32,
    -1.0f32, -1.0f32, -1.0f32,
    -1.0f32,  1.0f32, -1.0f32,
    -1.0f32,  1.0f32, -1.0f32,
    -1.0f32,  1.0f32,  1.0f32,
    -1.0f32, -1.0f32,  1.0f32,

    1.0f32, -1.0f32, -1.0f32,
    1.0f32, -1.0f32,  1.0f32,
    1.0f32,  1.0f32,  1.0f32,
    1.0f32,  1.0f32,  1.0f32,
    1.0f32,  1.0f32, -1.0f32,
    1.0f32, -1.0f32, -1.0f32,

    -1.0f32, -1.0f32,  1.0f32,
    -1.0f32,  1.0f32,  1.0f32,
    1.0f32,  1.0f32,  1.0f32,
    1.0f32,  1.0f32,  1.0f32,
    1.0f32, -1.0f32,  1.0f32,
    -1.0f32, -1.0f32,  1.0f32,

    -1.0f32,  1.0f32, -1.0f32,
    1.0f32,  1.0f32, -1.0f32,
    1.0f32,  1.0f32,  1.0f32,
    1.0f32,  1.0f32,  1.0f32,
    -1.0f32,  1.0f32,  1.0f32,
    -1.0f32,  1.0f32, -1.0f32,

    -1.0f32, -1.0f32, -1.0f32,
    -1.0f32, -1.0f32,  1.0f32,
    1.0f32, -1.0f32, -1.0f32,
    1.0f32, -1.0f32, -1.0f32,
    -1.0f32, -1.0f32,  1.0f32,
    1.0f32, -1.0f32,  1.0f32
];