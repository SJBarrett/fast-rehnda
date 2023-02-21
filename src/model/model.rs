use std::path::Path;
use std::sync::Arc;

use ash::vk;

use crate::core::{Vec2, Vec3};
use crate::etna::{Buffer, BufferCreateInfo, CommandPool, Device, PhysicalDevice, Texture};
use crate::model::Vertex;

pub struct Model {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub texture: Texture,
    pub index_count: u32,
}

impl Model {
    pub fn load_from_obj(device: Arc<Device>, physical_device: &PhysicalDevice, command_pool: &CommandPool, obj_path: &Path, texture_path: &Path) -> Model {
        let (models, _) = tobj::load_obj(obj_path, &tobj::GPU_LOAD_OPTIONS)
            .expect("Failed to load obj");
        if models.len() != 1 {
            panic!("Only expected 1 model in the obj file");
        }

        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u16> = Vec::new();
        for model in models {
            for index in 0..(model.mesh.positions.len() / 3) {
                let vertex_position = Vec3::new(
                    model.mesh.positions[index * 3],
                    model.mesh.positions[index * 3 + 1],
                    model.mesh.positions[index * 3 + 2],
                );
                let color = Vec3::new(1.0, 1.0, 1.0);
                let tex_coord = Vec2::new(model.mesh.texcoords[index * 2], 1.0 - model.mesh.texcoords[index * 2 + 1]);
                let vertex = Vertex {
                    position: vertex_position,
                    color,
                    texture_coord: tex_coord,
                };
                vertices.push(vertex);
            }

            indices = model.mesh.indices.iter().map(|&index| index as u16).collect();
        }
        let vertex_data: &[u8] = bytemuck::cast_slice(vertices.as_slice());
        let mut vertex_buffer = Buffer::create_empty_buffer(device.clone(), physical_device, BufferCreateInfo {
            size: vertex_data.len() as u64,
            usage: vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            memory_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
        });
        vertex_buffer.populate_buffer_using_staging_buffer(physical_device, command_pool, vertex_data);

        let index_buffer_data: &[u8] = bytemuck::cast_slice(indices.as_slice());
        let mut index_buffer = Buffer::create_empty_buffer(device.clone(), physical_device, BufferCreateInfo {
            size: index_buffer_data.len() as u64,
            usage: vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
            memory_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
        });
        index_buffer.populate_buffer_using_staging_buffer(physical_device, command_pool, index_buffer_data);

        let texture = Texture::create(device.clone(), physical_device, command_pool, texture_path);
        Model {
            vertex_buffer,
            index_buffer,
            texture,
            index_count: indices.len() as u32,
        }
    }

    pub fn create_from_vertices_and_indices(device: Arc<Device>, physical_device: &PhysicalDevice, command_pool: &CommandPool, vertices: &[Vertex], indices: &[u16], texture_path: &Path) -> Model {
        let buffer_data: &[u8] = bytemuck::cast_slice(vertices);
        let mut vertex_buffer = Buffer::create_empty_buffer(device.clone(), &physical_device, BufferCreateInfo {
            size: buffer_data.len() as u64,
            usage: vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            memory_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
        });
        vertex_buffer.populate_buffer_using_staging_buffer(&physical_device, &command_pool, buffer_data);

        let index_buffer_data: &[u8] = bytemuck::cast_slice(indices);
        let mut index_buffer = Buffer::create_empty_buffer(device.clone(), &physical_device, BufferCreateInfo {
            size: index_buffer_data.len() as u64,
            usage: vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
            memory_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
        });
        index_buffer.populate_buffer_using_staging_buffer(&physical_device, &command_pool, index_buffer_data);

        let texture = Texture::create(device, physical_device, command_pool, texture_path);

        Model {
            vertex_buffer,
            index_buffer,
            texture,
            index_count: indices.len() as u32,
        }
    }
}