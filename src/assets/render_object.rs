use std::mem::size_of;

use ash::vk;
use bevy_ecs::prelude::*;
use bytemuck_derive::{Pod, Zeroable};

use crate::etna::{Buffer, BufferCreateInfo, CommandPool, Device, Texture};
use crate::etna::material_pipeline::DescriptorManager;
use crate::rehnda_core::{ColorRgbaF, ConstPtr, Mat4};
use crate::assets::{AssetHandle, ModelHandle};
use crate::assets::material_server::MaterialPipelineHandle;

#[derive(Component)]
pub struct RenderObject {
    pub relative_transform: Mat4,
    pub model_handle: ModelHandle,
    pub material_handle: MaterialPipelineHandle,
}

impl RenderObject {
    pub fn new_with_transform(model_handle: ModelHandle, material_handle: MaterialPipelineHandle) -> RenderObject {
        RenderObject {
            relative_transform: Mat4::IDENTITY,
            model_handle,
            material_handle,
        }
    }
}

pub struct MultiMeshModel {
    pub meshes: Vec<Mesh>,
}

impl MultiMeshModel {
    pub fn with_single_mesh(mesh: Mesh) -> Self {
        MultiMeshModel {
            meshes: vec![mesh],
        }
    }
}

pub struct Mesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u32,
    pub relative_transform: Mat4,
    pub material_handle: MaterialHandle,
}

pub type MaterialHandle = AssetHandle<Material>;

pub enum Material {
    Standard(StdMaterial),
}

pub struct StdMaterial {
    pub base_color: ColorRgbaF,
    pub base_color_texture: Texture,
    pub normal_texture: Texture,
    pub occlusion_roughness_metal_texture: Texture,

    pub descriptor_set: vk::DescriptorSet,
    uniform_buffer: Buffer,
}

#[repr(C)]
#[derive(Pod, Zeroable, Debug, PartialEq, Copy, Clone)]
struct StdMaterialUniform {
    pub base_color: ColorRgbaF,
}

impl StdMaterial {
    pub fn create(device: ConstPtr<Device>, command_pool: &CommandPool, descriptor_manager: &mut DescriptorManager, base_color_texture: Texture, normal_texture: Texture, occlusion_roughness_metal_texture: Texture, base_color: ColorRgbaF) -> Self {
        let uniform = [StdMaterialUniform {
            base_color,
        }];
        let uniform_data: &[u8] = bytemuck::cast_slice(&uniform);
        let uniform_buffer = Buffer::create_and_initialize_buffer_with_staging_buffer(device, command_pool, BufferCreateInfo {
            data: uniform_data,
            usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
        });
        let material_props_buffer = vk::DescriptorBufferInfo::builder()
            .buffer(uniform_buffer.buffer)
            .offset(0)
            .range(size_of::<StdMaterialUniform>() as u64);
        let base_color_image_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(base_color_texture.image.image_view)
            .sampler(base_color_texture.sampler);
        let normal_image_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(normal_texture.image.image_view)
            .sampler(normal_texture.sampler);
        let occlusion_roughness_metal_image_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(occlusion_roughness_metal_texture.image.image_view)
            .sampler(occlusion_roughness_metal_texture.sampler);

        let (descriptor_set, _descriptor_set_layout) = descriptor_manager.descriptor_builder()
            .bind_image(0, base_color_image_info, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT)
            .bind_image(1, normal_image_info, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT)
            .bind_image(2, occlusion_roughness_metal_image_info, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT)
            .bind_buffer(3, material_props_buffer, vk::DescriptorType::UNIFORM_BUFFER, vk::ShaderStageFlags::FRAGMENT)
            .build()
            .expect("Failed to allocate bindings");
        Self {
            base_color_texture,
            normal_texture,
            occlusion_roughness_metal_texture,
            base_color,
            descriptor_set,
            uniform_buffer,
        }
    }
}