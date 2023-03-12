use std::{fs, io, mem};
use std::fmt::Debug;
use std::io::Read;
use std::mem::MaybeUninit;
use std::ops::Index;
use std::path::Path;

use ahash::AHashMap;
use ash::vk;
use bevy_ecs::prelude::info;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat};
use gltf::{Accessor, Gltf, Node, Primitive, Semantic};
use gltf::buffer;
use gltf::json::accessor::ComponentType;
use gltf::scene::Transform;
use image::{DynamicImage, EncodableLayout};
use log::info;

use crate::etna::{Buffer, BufferCreateInfo, CommandPool, Device, PhysicalDevice, SamplerOptions, TexSamplerOptions, Texture, TextureCreateInfo};
use crate::etna::material_pipeline::DescriptorManager;
use crate::rehnda_core::{ColorRgbaF, ConstPtr, Vec2, Vec3};
use crate::scene::Vertex;
use crate::scene::render_object::{Material, Mesh, MultiMeshModel, StdMaterial};

pub fn load_gltf(device: ConstPtr<Device>, physical_device: &PhysicalDevice, command_pool: &CommandPool, descriptor_manager: &mut DescriptorManager, gltf_path: &Path) -> MultiMeshModel {
    let working_dir = gltf_path.parent().unwrap();
    let gltf = read_gltf_file(gltf_path);
    let sources_data = SourcesData::load_data_into_memory(&gltf, working_dir);
    let mut meshes: Vec<Mesh> = Vec::new();
    for mesh in gltf.meshes() {
        for primitive in mesh.primitives() {
            meshes.push(build_mesh_from_primitives(device, physical_device, command_pool, descriptor_manager, &sources_data, primitive, &mesh));
        }
    }

    if let Some(scene) = gltf.scenes().nth(0) {
        for scene_node in scene.nodes() {
            update_transforms(&mut meshes, &scene_node, Mat4::IDENTITY);
        }
    }

    for mesh in &meshes {
        info!("Mesh {}: {:?}", mesh.name, mesh.relative_transform.to_scale_rotation_translation())
    }
    MultiMeshModel {
        meshes
    }
}

fn update_transforms(meshes: &mut Vec<Mesh>, node: &Node, parent_transform: Mat4) {
    let transform = parent_transform * gltf_transform_to_mat4(node.transform());
    if let Some(mesh) = node.mesh() {
        meshes[mesh.index()].relative_transform = transform;
    }
    for child_node in node.children() {
        update_transforms(meshes, &child_node, transform);
    }
}

fn gltf_transform_to_mat4(transform: Transform) -> Mat4 {
    match transform {
        Transform::Matrix { matrix } => Mat4::from_cols_array_2d(&matrix),
        Transform::Decomposed { translation, rotation, scale } => Mat4::from_scale_rotation_translation(scale.into(), Quat::from_array(rotation), translation.into())
    }
}

fn build_mesh_from_primitives(device: ConstPtr<Device>, physical_device: &PhysicalDevice, command_pool: &CommandPool, descriptor_manager: &mut DescriptorManager, data_buffers: &SourcesData, primitive: Primitive, gltf_mesh: &gltf::mesh::Mesh) -> Mesh {
    let material = primitive.material();
    let base_color_texture = material.pbr_metallic_roughness().base_color_texture();
    let base_color_tex_coord_index = base_color_texture.as_ref().map(|base_color_texture| base_color_texture.tex_coord());

    let primitive_attributes = PrimitiveAttributes::new(&primitive, data_buffers);

    let position_accessor: BufferAccessor<Vec3> = primitive_attributes.attribute_accessor(Semantic::Positions).unwrap();
    let normal_accessor: BufferAccessor<Vec3> = primitive_attributes.attribute_accessor(Semantic::Normals).unwrap();
    let base_color_tex_coord_accessor = base_color_tex_coord_index.and_then(|index| primitive_attributes.attribute_accessor::<Vec2>(Semantic::TexCoords(index)));

    let vertices: Vec<Vertex> = (0..primitive_attributes.vertex_count)
        .map(|i| {
            let mut position = position_accessor.data_at_index(i);
            Vertex {
                position,
                normal: normal_accessor.data_at_index(i),
                texture_coord: base_color_tex_coord_accessor.as_ref().map_or(Vec2::ZERO, |accessor| accessor.data_at_index(i)),
            }
        })
        .collect();

    let indices: Vec<u16> = (0..primitive_attributes.index_count)
        .map(|i| match &primitive_attributes.indices_accessor {
            IndexAccessor::U8(accessor) => {accessor.data_at_index(i) as u16}
            IndexAccessor::U16(accessor) => {accessor.data_at_index(i)}
        })
        .collect();


    let base_color_texture = base_color_texture.as_ref().map(|texture| {
        let image = data_buffers.images[texture.texture().index()].to_rgba8();
        let sampler_options = TexSamplerOptions::from_gltf(&texture.texture().sampler());

        Texture::create(device, physical_device, command_pool, descriptor_manager, &TextureCreateInfo {
            width: image.width(),
            height: image.height(),
            mip_levels: Some((image.width().max(image.height())).ilog2() + 1),
            data: image.as_bytes(),
            sampler_info: SamplerOptions::FilterOptions(&sampler_options),
        })
    }).unwrap_or_else(|| {
        let white_data = &ColorRgbaF::WHITE.to_rgba8();
        let white_img: &[u8] = bytemuck::cast_slice(white_data);
        Texture::create(device, physical_device, command_pool, descriptor_manager, &TextureCreateInfo {
            width: 1,
            height: 1,
            mip_levels: None,
            data: white_img,
            sampler_info: SamplerOptions::FilterOptions(&TexSamplerOptions {
                min_filter: None,
                mag_filter: None,
                mip_map_mode: None,
                address_mode_u: Default::default(),
                address_mode_v: Default::default(),
            }),
        })
    });

    let buffer_data: &[u8] = bytemuck::cast_slice(vertices.as_slice());
    let vertex_buffer = Buffer::create_and_initialize_buffer_with_staging_buffer(device, command_pool, BufferCreateInfo {
        data: buffer_data,
        usage: vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
    });

    let index_buffer_data: &[u8] = bytemuck::cast_slice(indices.as_slice());
    let index_buffer = Buffer::create_and_initialize_buffer_with_staging_buffer(device, command_pool, BufferCreateInfo {
        data: index_buffer_data,
        usage: vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
    });
    let base_color = ColorRgbaF::new_from_array(material.pbr_metallic_roughness().base_color_factor());
    let material = StdMaterial::create(device, command_pool, descriptor_manager, base_color_texture, base_color);

    Mesh {
        name: String::from(gltf_mesh.name().unwrap_or("defaultName")),
        vertex_buffer,
        index_buffer,
        material: Material::Standard(material),
        index_count: indices.len() as u32,
        relative_transform: Mat4::IDENTITY,
    }
}

struct PrimitiveAttributes<'a> {
    data_buffers: &'a SourcesData<'a>,
    semantic_accessors: AHashMap<Semantic, Accessor<'a>>,
    indices_accessor: IndexAccessor<'a>,
    vertex_count: usize,
    index_count: usize,
}

impl<'a> PrimitiveAttributes<'a> {
    fn new(primitive: &Primitive<'a>, data_buffers: &'a SourcesData) -> Self {
        let semantic_accessors: AHashMap<Semantic, Accessor<'a>> = primitive.attributes().map(|attribute| (attribute.0, attribute.1)).collect();
        let vertex_count = semantic_accessors.get(&Semantic::Positions).unwrap().count();
        let indices_accessor = match primitive.indices().unwrap().data_type() {
            ComponentType::U8 => {IndexAccessor::U8(BufferAccessor::new(data_buffers, &primitive.indices().unwrap()))}
            ComponentType::U16 => {IndexAccessor::U16(BufferAccessor::new(data_buffers, &primitive.indices().unwrap()))}
            _ => {panic!("Index type other than u8 and u16 are not supported")}
        };
        PrimitiveAttributes {
            semantic_accessors,
            data_buffers,
            indices_accessor,
            vertex_count,
            index_count: primitive.indices().unwrap().count(),
        }
    }

    fn attribute_accessor<T>(&self, semantic: Semantic) -> Option<BufferAccessor<'a, T>> where T: Pod, T: Zeroable {
        self.semantic_accessors.get(&semantic).map(|accessor| BufferAccessor::new(self.data_buffers, accessor))
    }
}

enum IndexAccessor<'a> {
    U8(BufferAccessor<'a, u8>),
    U16(BufferAccessor<'a, u16>),
}

enum BufferData<'a> {
    Source(SourceBuffers),
    Bin(&'a Vec<u8>),
}

struct SourceBuffers {
    data: Vec<u8>,
    buffer_offsets: Vec<usize>,
}

struct SourcesData<'a> {
    buffer_data: BufferData<'a>,
    images: Vec<DynamicImage>,
}

impl<'a> SourcesData<'a> {
    fn load_data_into_memory(gltf: &'a Gltf, working_dir: &Path) -> Self {
        let buffer_data_temp: Vec<MaybeUninit<u8>> = vec![MaybeUninit::<u8>::uninit(); gltf.buffers().map(|buffer| buffer.length()).sum()];
        let buffer_data = if let Some(bin) = &gltf.blob {
            BufferData::Bin(bin)
        } else {
            let mut buffer_data: Vec<u8> = unsafe { mem::transmute(buffer_data_temp) };
            let mut buffer_offsets: Vec<usize> = Vec::with_capacity(gltf.buffers().len());
            for buffer in gltf.buffers() {
                let offset = buffer_offsets.iter().sum();
                let buffer_end = offset + buffer.length();
                buffer_offsets.push(offset);
                match buffer.source() {
                    buffer::Source::Bin => {
                        unreachable!("Bin data is handled prior to this branch");
                    }
                    buffer::Source::Uri(uri) => {
                        let uri_path = working_dir.join(Path::new(uri));
                        let mut buffer_file = fs::File::open(uri_path).expect("Failed to open GLTF buffer source file");
                        buffer_file.read_exact(&mut buffer_data[offset..buffer_end]).expect("Failed to read buffer into vec");
                    }
                }
            }
            BufferData::Source(SourceBuffers {
                data: buffer_data,
                buffer_offsets,
            })
        };

        let mut images: Vec<DynamicImage> = Vec::with_capacity(gltf.images().len());
        for image in gltf.images() {
            let image = match image.source() {
                gltf::image::Source::View {
                    view,
                    mime_type,
                } => {
                    match &buffer_data {
                        BufferData::Source(_) => {
                            todo!()
                        }
                        BufferData::Bin(data) => {
                            let data = &data[view.offset()..view.offset() + view.length()];
                            image::load_from_memory(data).expect("Failed to build image from Bin data")
                        }
                    }
                }
                gltf::image::Source::Uri {
                    uri,
                    mime_type,
                } => {
                    let decoded = urlencoding::decode(uri).unwrap();
                    let img_path = working_dir.join(Path::new(decoded.as_ref()));
                    image::open(img_path).expect("Failed to open gltf image")
                }
            };
            images.push(image);
        }

        SourcesData {
            buffer_data,
            images,
        }
    }

    fn buffer_ref(&self, index: usize) -> &[u8] {
        match &self.buffer_data {
            BufferData::Source(sources_data) => {
                let offset = sources_data.buffer_offsets[index];
                let end_of_buffer = if index == sources_data.buffer_offsets.len() - 1 {
                    sources_data.data.len()
                } else {
                    sources_data.buffer_offsets[index + 1]
                };
                &sources_data.data[offset..end_of_buffer]
            }
            BufferData::Bin(bin) => {
                bin.as_slice()
            }
        }
    }
}

struct BufferAccessor<'a, T> where T: Zeroable, T: Pod {
    pub buffer_data: &'a [u8],
    pub stride: usize,
    pub offset: usize,
    marker: std::marker::PhantomData<T>,
}

impl<'a, T> BufferAccessor<'a, T> where T: Zeroable, T: Pod {
    fn for_attribute(semantic: Semantic, attributes_map: &AHashMap<Semantic, Accessor>, data_buffers: &'a SourcesData) -> Option<Self> {
        let accessor = attributes_map.get(&semantic);
        accessor.map(|acc| BufferAccessor::<T>::new(data_buffers, acc))
    }

    fn new(buffers: &'a SourcesData, accessor: &Accessor) -> Self {
        let view = accessor.view().unwrap();
        let stride = view.stride().unwrap_or_else(|| accessor.size());
        let offset = accessor.offset() + view.offset();
        BufferAccessor {
            buffer_data: buffers.buffer_ref(view.buffer().index()),
            stride,
            offset,
            marker: std::marker::PhantomData,
        }
    }

    fn data_at_index(&self, index: usize) -> T {
        let data = unsafe { self.buffer_data.as_ptr().add(self.offset + index * self.stride) };
        unsafe { *(data as *const T) }
    }
}

fn read_gltf_file(path: &Path) -> Gltf {
    let file = fs::File::open(path).expect("failed to open gltf file");
    let reader = io::BufReader::new(file);
    Gltf::from_reader(reader).expect("Failed to read gltf")
}