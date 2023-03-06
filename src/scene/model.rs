use std::{fs, io};
use std::fmt::Debug;
use std::io::Read;
use std::path::Path;
use ahash::AHashMap;

use ash::vk;
use bevy_ecs::prelude::Component;
use bytemuck::{Pod, Zeroable};
use gltf::{Accessor, Gltf, Semantic};
use gltf::buffer::Source;
use gltf::json::accessor::{ComponentType, Type};
use log::info;

use crate::rehnda_core::{ConstPtr, Mat4, Vec2, Vec3};
use crate::etna::{Buffer, BufferCreateInfo, CommandPool, Device, PhysicalDevice, Texture};
use crate::etna::material_pipeline::DescriptorManager;
use crate::scene::{MaterialHandle, ModelHandle, Vertex};

#[derive(Component)]
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

    pub fn create_from_vertices_and_indices(device: ConstPtr<Device>, command_pool: &CommandPool, vertices: &[Vertex], indices: &[u16]) -> Model {
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

        Model {
            vertex_buffer,
            index_buffer,
            texture: None,
            index_count: indices.len() as u32,
        }
    }

    pub fn load_gltf(device: ConstPtr<Device>, physical_device: &PhysicalDevice, command_pool: &CommandPool, descriptor_manager: &mut DescriptorManager, gltf_path: &Path) -> Model {
        let working_dir = gltf_path.parent().unwrap();
        let gltf = read_gltf_file(gltf_path);
        let gltf_buffers = GltfBuffers::load_data_into_memory(&gltf, working_dir);
        let mut models: Vec<Model> = Vec::new();
        for mesh in gltf.meshes() {
            for primitive in mesh.primitives() {
                let attributes: AHashMap<Semantic, Accessor> = primitive.attributes().map(|attribute| (attribute.0, attribute.1)).collect();
                let position_attribute = attributes.get(&Semantic::Positions).unwrap();
                let normal_attribute = attributes.get(&Semantic::Normals).unwrap();
                let indices_attribute = primitive.indices().expect("Expected indices to be present for a GLTF mesh");
                let position_accessor: BufferAccessor<Vec3> = BufferAccessor::new(&gltf_buffers, &position_attribute);
                let normal_accessor: BufferAccessor<Vec3> = BufferAccessor::new(&gltf_buffers, &normal_attribute);
                let vertices: Vec<Vertex> = (0..position_attribute.count())
                    .map(|i| Vertex {
                        position: position_accessor.data_at_index(i),
                        normal: normal_accessor.data_at_index(i),
                        texture_coord: Vec2::ZERO,
                    })
                    .collect();
                let index_accessor: BufferAccessor<u16> = BufferAccessor::new(&gltf_buffers, &indices_attribute);
                let indices: Vec<u16> = (0..indices_attribute.count())
                    .map(|i| index_accessor.data_at_index(i))
                    .collect();
                models.push(Model::create_from_vertices_and_indices(device, command_pool, vertices.as_slice(), indices.as_slice()));
            }
        }
        info!("Loaded gltf model with {} meshes", gltf.meshes().len());
        models.remove(0)
    }
}

struct GltfBuffers {
    buffer_data: Vec<Vec<u8>>,
}

impl GltfBuffers {
    fn load_data_into_memory(gltf: &Gltf, working_dir: &Path) -> Self {
        let mut buffer_sources: Vec<Vec<u8>> = Vec::with_capacity(gltf.buffers().len());
        unsafe {
            for buffer in gltf.buffers() {
                let mut buffer_data: Vec<u8> = vec![0; buffer.length()];
                match buffer.source() {
                    Source::Bin => {
                        todo!("Support BIN buffer source for gltf")
                    }
                    Source::Uri(uri) => {
                        let uri_path = working_dir.join(Path::new(uri));
                        let mut buffer_file = fs::File::open(uri_path).expect("Failed to open GLTF buffer source file");
                        buffer_file.read_exact(buffer_data.as_mut_slice()).expect("Failed to read buffer into vec");
                    }
                }
                buffer_sources.insert(buffer.index(), buffer_data);
            }
        }
        GltfBuffers {
            buffer_data: buffer_sources
        }
    }

    fn buffer_ref(&self, index: usize) -> &[u8] {
        self.buffer_data[index].as_slice()
    }
}

struct BufferAccessor<'a, T> where T: Zeroable, T: Pod  {
    pub buffer_data: &'a[u8],
    pub stride: usize,
    pub offset: usize,
    marker: std::marker::PhantomData<T>
}

impl<'a, T> BufferAccessor<'a, T> where T: Zeroable, T: Pod  {
    fn new(buffers: &'a GltfBuffers, accessor: &Accessor) -> Self {
        let view = accessor.view().unwrap();
        let stride = view.stride().unwrap_or_else(|| accessor.size());
        let offset = accessor.offset() + view.offset();
        BufferAccessor {
            buffer_data: buffers.buffer_ref(view.buffer().index()),
            stride,
            offset,
            marker: std::marker::PhantomData
        }
    }

    fn data_at_index(&self, index: usize) -> T  {
        let data = unsafe { self.buffer_data.as_ptr().add(self.offset + index * self.stride) };
        unsafe { *(data as *const T) }
    }
}

fn read_gltf_file(path: &Path) -> Gltf {
    let file = fs::File::open(path).expect("failed to open gltf file");
    let reader = io::BufReader::new(file);
    Gltf::from_reader(reader).expect("Failed to read gltf")
}