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
use glam::{Mat4, Quat, Vec4Swizzles};
use gltf::{Accessor, Gltf, Node, Semantic};
use gltf::buffer;
use gltf::json::accessor::ComponentType;
use gltf::material::NormalTexture;
use gltf::scene::Transform;
use image::{DynamicImage, EncodableLayout, RgbaImage};
use lazy_static::lazy_static;
use log::info;

use crate::etna::{Buffer, BufferCreateInfo, CommandPool, Device, PhysicalDevice, SamplerOptions, TexSamplerOptions, Texture, TextureCreateInfo};
use crate::etna::material_pipeline::DescriptorManager;
use crate::rehnda_core::{ColorRgbaF, ConstPtr, Vec2, Vec3, Vec4};
use crate::assets::render_object::{Material, MaterialHandle, Mesh, StdMaterial};
use crate::assets::Vertex;

lazy_static! {
    static ref MISSING_TEXTURE_IMG: RgbaImage = missing_texture();
}

pub type MeshesAndMaterials = (Vec<Mesh>, Vec<Material>, Vec<usize>);

pub fn load_gltf(device: ConstPtr<Device>, physical_device: &PhysicalDevice, command_pool: &CommandPool, descriptor_manager: &mut DescriptorManager, gltf_path: &Path) -> MeshesAndMaterials {
    let working_dir = gltf_path.parent().unwrap();
    let gltf = read_gltf_file(gltf_path);
    let sources_data = SourcesData::load_data_into_memory(&gltf, working_dir);
    let materials: Vec<Material> = gltf.materials()
        .map(|gltf_material| load_gltf_material(device, physical_device, command_pool, descriptor_manager, &sources_data, &gltf_material))
        .collect();
    let mut meshes: Vec<Mesh> = Vec::new();
    let mut mesh_material_indices: Vec<usize> = Vec::new();
    for gltf_mesh in gltf.meshes() {
        for primitive in gltf_mesh.primitives() {
            mesh_material_indices.push(primitive.material().index().unwrap());
            meshes.push(build_mesh_from_primitives(device, command_pool, &sources_data, primitive));
        }
    }

    if let Some(scene) = gltf.scenes().next() {
        for scene_node in scene.nodes() {
            update_transforms(&mut meshes, &scene_node, Mat4::IDENTITY);
        }
    }

    (meshes, materials, mesh_material_indices)
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

fn missing_texture() -> image::RgbaImage {
    let img_path = Path::new("assets/debug/missing_texture_image.jpg");
    image::open(img_path).expect("Failed to open gltf image").to_rgba8()
}

fn default_texture(device: ConstPtr<Device>, physical_device: &PhysicalDevice, command_pool: &CommandPool, descriptor_manager: &mut DescriptorManager) -> Texture {
    Texture::create(device, physical_device, command_pool, descriptor_manager, &TextureCreateInfo {
        width: MISSING_TEXTURE_IMG.width(),
        height: MISSING_TEXTURE_IMG.height(),
        mip_levels: Some((MISSING_TEXTURE_IMG.width().max(MISSING_TEXTURE_IMG.height())).ilog2() + 1),
        data: MISSING_TEXTURE_IMG.as_bytes(),
        sampler_info: SamplerOptions::FilterOptions(&TexSamplerOptions {
            min_filter: None,
            mag_filter: None,
            mip_map_mode: None,
            address_mode_u: Default::default(),
            address_mode_v: Default::default(),
        }),
        format: vk::Format::R8G8B8A8_SRGB,
    })
}

fn load_gltf_material(device: ConstPtr<Device>, physical_device: &PhysicalDevice, command_pool: &CommandPool, descriptor_manager: &mut DescriptorManager, data_buffers: &SourcesData, gltf_material: &gltf::material::Material) -> Material {
    let base_color_texture = gltf_material.pbr_metallic_roughness().base_color_texture();
    let base_color_tex_coord_index = base_color_texture.as_ref().map(|base_color_texture| base_color_texture.tex_coord());
    assert_eq!(base_color_tex_coord_index.unwrap(), 0, "Currently only support loading gltf models with the attribute TEXCOORD_0");
    let base_color = ColorRgbaF::new_from_array(gltf_material.pbr_metallic_roughness().base_color_factor());

    let base_color_texture = base_color_texture.as_ref().map(|texture| {
        load_gltf_texture(device, physical_device, command_pool, descriptor_manager, data_buffers, &texture.texture(), vk::Format::R8G8B8A8_SRGB)
    }).unwrap_or_else(|| {
        default_texture(device, physical_device, command_pool, descriptor_manager)
    });

    let normal_texture = gltf_material.normal_texture().map(|texture| {
        load_gltf_texture(device, physical_device, command_pool, descriptor_manager, data_buffers, &texture.texture(), vk::Format::R8G8B8A8_UNORM)
    }).unwrap_or_else(|| {
        default_texture(device, physical_device, command_pool, descriptor_manager)
    });

    // TODO this assumes that occlusion always uses the R channel, metal B and roughness G. Metal and
    // roughness are always together, but not necessarily occlusion
    let occlusion_roughness_metallic_texture = gltf_material.pbr_metallic_roughness().metallic_roughness_texture().map(|texture| {
        load_gltf_texture(device, physical_device, command_pool, descriptor_manager, data_buffers, &texture.texture(), vk::Format::R8G8B8A8_UNORM)
    }).unwrap_or_else(|| {
        default_texture(device, physical_device, command_pool, descriptor_manager)
    });

    let material = StdMaterial::create(device, command_pool, descriptor_manager, base_color_texture, normal_texture, occlusion_roughness_metallic_texture, base_color);
    Material::Standard(material)
}

fn build_mesh_from_primitives(device: ConstPtr<Device>, command_pool: &CommandPool, data_buffers: &SourcesData, primitive: gltf::Primitive) -> Mesh {
    let primitive_attributes = PrimitiveAttributes::new(&primitive, data_buffers);

    let position_accessor: BufferAccessor<Vec3> = primitive_attributes.attribute_accessor(Semantic::Positions).unwrap();
    // TODO handle when no tangents exist on a model
    let tangent_accessor: BufferAccessor<[f32; 4]> = primitive_attributes.attribute_accessor(Semantic::Tangents).unwrap();
    let normal_accessor: BufferAccessor<Vec3> = primitive_attributes.attribute_accessor(Semantic::Normals).unwrap();
    let base_color_tex_coord_accessor: BufferAccessor<Vec2> = primitive_attributes.attribute_accessor(Semantic::TexCoords(0)).unwrap();

    let vertices: Vec<Vertex> = (0..primitive_attributes.vertex_count)
        .map(|i| {
            let position = position_accessor.data_at_index(i);
            let tangent = tangent_accessor.data_at_index(i);
            let normal = normal_accessor.data_at_index(i);
            Vertex {
                position,
                normal,
                texture_coord: base_color_tex_coord_accessor.data_at_index(i),
                tangent: Vec4::new(tangent[0], tangent[1], tangent[2], tangent[3]),
            }
        })
        .collect();

    let indices: Vec<u32> = (0..primitive_attributes.index_count)
        .map(|i| match &primitive_attributes.indices_accessor {
            IndexAccessor::U8(accessor) => { accessor.data_at_index(i) as u32 }
            IndexAccessor::U16(accessor) => { accessor.data_at_index(i) as u32 }
            IndexAccessor::U32(accessor) => { accessor.data_at_index(i) }
        })
        .collect();

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

    Mesh {
        vertex_buffer,
        index_buffer,
        index_count: indices.len() as u32,
        relative_transform: Mat4::IDENTITY,
        material_handle: MaterialHandle::null(),
    }
}

fn load_gltf_texture(device: ConstPtr<Device>, physical_device: &PhysicalDevice, command_pool: &CommandPool, descriptor_manager: &mut DescriptorManager, data_buffers: &SourcesData, texture: &gltf::Texture, format: vk::Format) -> Texture {
    let image = data_buffers.images[texture.index()].to_rgba8();
    let sampler_options = TexSamplerOptions::from_gltf(&texture.sampler());

    Texture::create(device, physical_device, command_pool, descriptor_manager, &TextureCreateInfo {
        width: image.width(),
        height: image.height(),
        mip_levels: Some((image.width().max(image.height())).ilog2() + 1),
        data: image.as_bytes(),
        sampler_info: SamplerOptions::FilterOptions(&sampler_options),
        format,
    })
}

struct PrimitiveAttributes<'a> {
    data_buffers: &'a SourcesData<'a>,
    semantic_accessors: AHashMap<Semantic, Accessor<'a>>,
    indices_accessor: IndexAccessor<'a>,
    vertex_count: usize,
    index_count: usize,
}

impl<'a> PrimitiveAttributes<'a> {
    fn new(primitive: &gltf::Primitive<'a>, data_buffers: &'a SourcesData) -> Self {
        let semantic_accessors: AHashMap<Semantic, Accessor<'a>> = primitive.attributes().map(|attribute| (attribute.0, attribute.1)).collect();
        let vertex_count = semantic_accessors.get(&Semantic::Positions).unwrap().count();
        let indices_accessor = match primitive.indices().unwrap().data_type() {
            ComponentType::U8 => { IndexAccessor::U8(BufferAccessor::new(data_buffers, &primitive.indices().unwrap())) }
            ComponentType::U16 => { IndexAccessor::U16(BufferAccessor::new(data_buffers, &primitive.indices().unwrap())) }
            ComponentType::U32 => { IndexAccessor::U32(BufferAccessor::new(data_buffers, &primitive.indices().unwrap())) }
            _ => { panic!("Index type other than u8, u16 and u32 are not supported") }
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
    U32(BufferAccessor<'a, u32>),
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