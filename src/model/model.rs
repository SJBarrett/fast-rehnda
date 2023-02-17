use std::sync::Arc;
use ash::vk;
use crate::etna;
use crate::etna::{BufferCreateInfo, CommandPool, Device, PhysicalDevice};
use crate::model::Vertex;

pub struct Model {
    pub vertex_buffer: etna::Buffer,
    pub index_buffer: etna::Buffer,
}

impl Model {
    pub fn create_from_vertices_and_indices(device: Arc<Device>, physical_device: &PhysicalDevice, command_pool: &CommandPool, vertices: &[Vertex], indices: &[u16]) -> Model {
        let buffer_data: &[u8] = bytemuck::cast_slice(vertices);
        let mut vertex_buffer = etna::Buffer::create_empty_buffer(device.clone(), &physical_device, BufferCreateInfo {
            size: buffer_data.len() as u64,
            usage: vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            memory_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
        });
        vertex_buffer.populate_buffer_using_staging_buffer(&physical_device, &command_pool, buffer_data);

        let index_buffer_data: &[u8] = bytemuck::cast_slice(indices);
        let mut index_buffer = etna::Buffer::create_empty_buffer(device.clone(), &physical_device, BufferCreateInfo {
            size: index_buffer_data.len() as u64,
            usage: vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
            memory_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
        });
        index_buffer.populate_buffer_using_staging_buffer(&physical_device, &command_pool, index_buffer_data);

        Model {
            vertex_buffer,
            index_buffer,
        }
    }
}