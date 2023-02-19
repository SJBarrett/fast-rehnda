use std::mem::size_of;
use ash::vk;
use memoffset::offset_of;
use crate::core::*;
use bytemuck_derive::{Zeroable, Pod};

pub const TRIANGLE_VERTICES: [Vertex; 8] = [
    Vertex { position: Vec3::new(-0.5, -0.5, 0.0), color: ColorRgb::new(1.0, 0.0, 0.0), texture_coord: Vec2::new(1.0, 0.0) },
    Vertex { position: Vec3::new(0.5, -0.5, 0.0), color: ColorRgb::new(0.0, 1.0, 0.0), texture_coord: Vec2::new(0.0, 0.0) },
    Vertex { position: Vec3::new(0.5, 0.5, 0.0), color: ColorRgb::new(0.0, 0.0, 1.0), texture_coord: Vec2::new(0.0, 1.0) },
    Vertex { position: Vec3::new(-0.5, 0.5, 0.0), color: ColorRgb::new(1.0, 1.0, 1.0), texture_coord: Vec2::new(1.0, 1.0) },
    // second square
    Vertex { position: Vec3::new(-0.5, -0.5, -0.5), color: ColorRgb::new(1.0, 0.0, 0.0), texture_coord: Vec2::new(1.0, 0.0) },
    Vertex { position: Vec3::new(0.5, -0.5, -0.5), color: ColorRgb::new(0.0, 1.0, 0.0), texture_coord: Vec2::new(0.0, 0.0) },
    Vertex { position: Vec3::new(0.5, 0.5, -0.5), color: ColorRgb::new(0.0, 0.0, 1.0), texture_coord: Vec2::new(0.0, 1.0) },
    Vertex { position: Vec3::new(-0.5, 0.5, -0.5), color: ColorRgb::new(1.0, 1.0, 1.0), texture_coord: Vec2::new(1.0, 1.0) },
];

pub const TRIANGLE_INDICES: [u16; 12] = [
    0, 1, 2, 2, 3, 0,
    4, 5, 6, 6, 7, 4,
];

#[repr(C)]
#[derive(Zeroable, Pod, Debug, Copy, Clone)]
pub struct Vertex {
    pub position: Vec3,
    pub color: Vec3,
    pub texture_coord: Vec2,
}

impl Vertex {
    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            // position attribute
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Vertex, position) as u32)
                .build(),
            // color attribute
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Vertex, color) as u32)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(2)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(offset_of!(Vertex, texture_coord) as u32)
                .build(),
        ]
    }
}