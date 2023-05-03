use ash::vk;
use bevy_ecs::prelude::*;
use crevice::std140::AsStd140;
use glam::Vec4Swizzles;
use crate::assets::demo_scenes::Actor;
use crate::assets::render_object::Transform;
use crate::etna::{Device, HostMappedBuffer, HostMappedBufferCreateInfo};
use crate::etna::material_pipeline::DescriptorManager;
use crate::rehnda_core::{ConstPtr, Vec3};

#[derive(Component)]
pub struct PointLight {
    pub light_color: Vec3,
    pub emissivity: f32,
}

#[derive(AsStd140)]
struct PointLightUniform {
    pub position: Vec3,
    pub light_color: Vec3,
    pub emissivity: f32,
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            light_color: (1.0, 1.0, 1.0).into(),
            emissivity: 1.0,
        }
    }
}


#[derive(Resource)]
pub struct LightingDataManager {
    pub point_light_buffer: HostMappedBuffer,
    pub descriptor_set: vk::DescriptorSet,
}

impl LightingDataManager {
    pub fn new(device: ConstPtr<Device>, descriptor_manager: &mut DescriptorManager) -> Self {
        let buffer = HostMappedBuffer::create(device, HostMappedBufferCreateInfo {
           size: PointLightUniform::std140_size_static() as u64,
            usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
        });
        let descriptor_buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(buffer.vk_buffer())
            .offset(0)
            .range(PointLightUniform::std140_size_static() as u64);
        let (descriptor_set, _) = descriptor_manager.descriptor_builder()
            .bind_buffer(0, descriptor_buffer_info, vk::DescriptorType::UNIFORM_BUFFER, vk::ShaderStageFlags::FRAGMENT)
            .build()
            .expect("Failed to build light buffer");
        Self {
            point_light_buffer: buffer,
            descriptor_set,
        }
    }
}

pub fn update_lights_system(mut lighting_data_manager: ResMut<LightingDataManager>, lights: Query<(&Transform, &PointLight)>) {
    if let Some((transform, light)) = lights.iter().nth(0) {
        let light_uniform = PointLightUniform {
            position: transform.translation,
            light_color: light.light_color,
            emissivity: light.emissivity,
        }.as_std140();
        lighting_data_manager.point_light_buffer.write_data(light_uniform.as_bytes());
    }
}