use std::mem::size_of;

use ash::vk;
use bevy_ecs::prelude::*;
use bytemuck_derive::{Pod, Zeroable};

use crate::etna::{Buffer, BufferCreateInfo, CommandPool, Device, Texture};
use crate::etna::material_pipeline::DescriptorManager;
use crate::rehnda_core::{ColorRgbaF, ConstPtr, Mat4, Quat, Vec3};
use crate::assets::{AssetHandle, MeshHandle};
use crate::assets::material_server::MaterialPipelineHandle;

#[derive(Component)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

#[derive(Component, Copy, Clone)]
pub struct RenderObject {
    pub mesh_handle: MeshHandle,
    pub material_instance_handle: MaterialHandle,
    pub material_pipeline_handle: MaterialPipelineHandle,
}

pub struct Mesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u32,
    pub relative_transform: Mat4,
}

pub type MaterialHandle = AssetHandle<PbrMaterial>;

pub struct PbrMaterial {
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

impl PbrMaterial {
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