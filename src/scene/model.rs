use std::path::Path;

use ash::vk;

use crate::rehnda_core::{ConstPtr, Mat4, Vec2, Vec3};
use crate::etna::{Buffer, BufferCreateInfo, CommandPool, Device, PhysicalDevice, Texture};
use crate::etna::material_pipeline::DescriptorManager;
use crate::scene::{MaterialHandle, ModelHandle, Vertex};

pub struct RenderObject {
    pub transform: Mat4,
    pub model_handle: ModelHandle,
    pub material_handle: MaterialHandle,
}

impl RenderObject {
    pub fn new_with_transform(transform: Mat4, model_handle: ModelHandle, material_handle: MaterialHandle) -> RenderObject {
        RenderObject {
            transform,
            model_handle,
            material_handle,
        }
    }
}

pub struct Model {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub texture: Option<Texture>,
    pub index_count: u32,
}

impl Model {
    pub fn load_textured_obj(device: ConstPtr<Device>, physical_device: &PhysicalDevice, command_pool: &CommandPool, descriptor_manager: &mut DescriptorManager, obj_path: &Path, texture_path: &Path) -> Model {
        let (index_count, vertex_buffer, index_buffer) = Self::load_obj_vertices_and_indices(device, command_pool, obj_path);

        let texture = Texture::create_from_image_file(device, physical_device, command_pool, texture_path, descriptor_manager);
        Model {
            vertex_buffer,
            index_buffer,
            index_count,
            texture: Some(texture),
        }
    }

    pub fn load_obj(device: ConstPtr<Device>, command_pool: &CommandPool, obj_path: &Path) -> Model {
        let (index_count, vertex_buffer, index_buffer) = Self::load_obj_vertices_and_indices(device, command_pool, obj_path);

        Model {
            vertex_buffer,
            index_buffer,
            index_count,
            texture: None,
        }
    }

    fn load_obj_vertices_and_indices(device: ConstPtr<Device>, command_pool: &CommandPool, obj_path: &Path) -> (u32, Buffer, Buffer) {
        let (models, _) = tobj::load_obj(obj_path, &tobj::GPU_LOAD_OPTIONS)
            .expect("Failed to load obj");
        if models.len() != 1 {
            panic!("Only expected 1 scene in the obj file");
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
                let tex_coord = Vec2::new(model.mesh.texcoords[index * 2], 1.0 - model.mesh.texcoords[index * 2 + 1]);
                let normal = Vec3::new(
                    model.mesh.normals[index * 3],
                    model.mesh.normals[index * 3 + 1],
                    model.mesh.normals[index * 3 + 2],
                );
                let vertex = Vertex {
                    position: vertex_position,
                    normal,
                    texture_coord: tex_coord,
                };
                vertices.push(vertex);
            }

            indices = model.mesh.indices.iter().map(|&index| index as u16).collect();
        }
        let vertex_data: &[u8] = bytemuck::cast_slice(vertices.as_slice());
        let vertex_buffer = Buffer::create_and_initialize_buffer_with_staging_buffer(device, command_pool, BufferCreateInfo {
            data: vertex_data,
            usage: vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
        });

        let index_buffer_data: &[u8] = bytemuck::cast_slice(indices.as_slice());
        let index_buffer = Buffer::create_and_initialize_buffer_with_staging_buffer(device, command_pool, BufferCreateInfo {
            data: index_buffer_data,
            usage: vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
        });
        (indices.len() as u32, vertex_buffer, index_buffer)
    }

    pub fn create_from_vertices_and_indices(device: ConstPtr<Device>, physical_device: &PhysicalDevice, command_pool: &CommandPool, descriptor_manager: &mut DescriptorManager, vertices: &[Vertex], indices: &[u16], texture_path: &Path) -> Model {
        let buffer_data: &[u8] = bytemuck::cast_slice(vertices);
        let vertex_buffer = Buffer::create_and_initialize_buffer_with_staging_buffer(device, command_pool, BufferCreateInfo {
            data: buffer_data,
            usage: vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
        });

        let index_buffer_data: &[u8] = bytemuck::cast_slice(indices);
        let index_buffer = Buffer::create_and_initialize_buffer_with_staging_buffer(device, command_pool, BufferCreateInfo {
            data: index_buffer_data,
            usage: vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
        });

        let texture = Texture::create_from_image_file(device, physical_device, command_pool, texture_path, descriptor_manager);

        Model {
            vertex_buffer,
            index_buffer,
            texture: Some(texture),
            index_count: indices.len() as u32,
        }
    }
}