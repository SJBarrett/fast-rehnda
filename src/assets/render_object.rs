use std::mem::size_of;
use std::sync::Arc;

use ash::vk;
use bevy_ecs::prelude::*;
use bytemuck_derive::{Pod, Zeroable};
use enumflags2::{BitFlag, bitflags, BitFlags};

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
    options: PbrMaterialOptions,
    textures: Arc<PbrMaterialTextures>,
    descriptor_set: vk::DescriptorSet,
    uniform_buffer: Buffer,
}


#[bitflags]
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PbrMaterialFeatureFlags {
    AlbedoTexture = 1 << 0,
    NormalTexture = 1 << 1,
    RoughnessTexture = 1 << 2,
    MetallicTexture = 1 << 3,
    OcclusionTexture = 1 << 4,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PbrMaterialOptions {
    pub base_color: ColorRgbaF,
    pub roughness: f32,
    pub metallic: f32,
    pub features: BitFlags<PbrMaterialFeatureFlags>,
}

impl Default for PbrMaterialOptions {
    fn default() -> Self {
        Self {
            base_color: ColorRgbaF::WHITE,
            roughness: 1.0,
            metallic: 1.0,
            features: PbrMaterialFeatureFlags::empty(),
        }
    }
}

#[repr(C)]
#[derive(Pod, Zeroable, Debug, PartialEq, Copy, Clone)]
pub struct PbrMaterialUniforms {
    pub base_color: ColorRgbaF,
    pub roughness: f32,
    pub metallic: f32,
    pub enabled_feature_flags: u32,
}

impl PbrMaterialUniforms {
    fn from_options(options: &PbrMaterialOptions) -> Self {
        Self {
            base_color: options.base_color,
            roughness: options.roughness,
            metallic: options.metallic,
            enabled_feature_flags: options.features.bits(),
        }
    }
}


pub struct PbrMaterialTextures {
    pub base_color_texture: Texture,
    pub normal_texture: Texture,
    pub occlusion_roughness_metallic_texture: Texture,
}

impl PbrMaterial {
    pub fn descriptor_set(&self) -> vk::DescriptorSet {
        self.descriptor_set
    }

    pub fn create(device: ConstPtr<Device>, command_pool: &CommandPool, descriptor_manager: &mut DescriptorManager, textures: Arc<PbrMaterialTextures>, options: &PbrMaterialOptions) -> Self {
        let uniform = [PbrMaterialUniforms::from_options(options)];
        let uniform_data: &[u8] = bytemuck::cast_slice(&uniform);
        let uniform_buffer = Buffer::create_and_initialize_buffer_with_staging_buffer(device, command_pool, BufferCreateInfo {
            data: uniform_data,
            usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
        });
        let material_props_buffer = vk::DescriptorBufferInfo::builder()
            .buffer(uniform_buffer.buffer)
            .offset(0)
            .range(size_of::<PbrMaterialUniforms>() as u64);
        let base_color_image_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(textures.base_color_texture.image.image_view)
            .sampler(textures.base_color_texture.sampler);
        let normal_image_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(textures.normal_texture.image.image_view)
            .sampler(textures.normal_texture.sampler);
        let occlusion_roughness_metal_image_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(textures.occlusion_roughness_metallic_texture.image.image_view)
            .sampler(textures.occlusion_roughness_metallic_texture.sampler);

        let (descriptor_set, _descriptor_set_layout) = descriptor_manager.descriptor_builder()
            .bind_buffer(0, material_props_buffer, vk::DescriptorType::UNIFORM_BUFFER, vk::ShaderStageFlags::FRAGMENT)
            .bind_image(1, base_color_image_info, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT)
            .bind_image(2, normal_image_info, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT)
            .bind_image(3, occlusion_roughness_metal_image_info, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT)
            .build()
            .expect("Failed to allocate bindings");
        Self {
            textures,
            options: *options,
            descriptor_set,
            uniform_buffer,
        }
    }

    pub fn copy_with_new_uniforms(&self, device: ConstPtr<Device>, command_pool: &CommandPool, descriptor_manager: &mut DescriptorManager, options: &PbrMaterialOptions) -> Self {
        let uniform = [PbrMaterialUniforms::from_options(options)];
        let uniform_data: &[u8] = bytemuck::cast_slice(&uniform);
        let uniform_buffer = Buffer::create_and_initialize_buffer_with_staging_buffer(device, command_pool, BufferCreateInfo {
            data: uniform_data,
            usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
        });
        let material_props_buffer = vk::DescriptorBufferInfo::builder()
            .buffer(uniform_buffer.buffer)
            .offset(0)
            .range(size_of::<PbrMaterialUniforms>() as u64);
        let base_color_image_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(self.textures.base_color_texture.image.image_view)
            .sampler(self.textures.base_color_texture.sampler);
        let normal_image_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(self.textures.normal_texture.image.image_view)
            .sampler(self.textures.normal_texture.sampler);
        let occlusion_roughness_metal_image_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(self.textures.occlusion_roughness_metallic_texture.image.image_view)
            .sampler(self.textures.occlusion_roughness_metallic_texture.sampler);

        let (descriptor_set, _descriptor_set_layout) = descriptor_manager.descriptor_builder()
            .bind_buffer(0, material_props_buffer, vk::DescriptorType::UNIFORM_BUFFER, vk::ShaderStageFlags::FRAGMENT)
            .bind_image(1, base_color_image_info, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT)
            .bind_image(2, normal_image_info, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT)
            .bind_image(3, occlusion_roughness_metal_image_info, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT)
            .build()
            .expect("Failed to allocate bindings");
        Self {
            textures: self.textures.clone(),
            options: *options,
            descriptor_set,
            uniform_buffer,
        }
    }
}