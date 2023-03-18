use ash::vk;
use bevy_ecs::system::Resource;
use crate::rehnda_core::ConstPtr;
use crate::etna::Device;
use crate::etna::material_pipeline::{layout_binding, DescriptorAllocator, DescriptorBuilder, DescriptorLayoutCache};

#[derive(Resource)]
pub struct DescriptorManager {
    pub allocator: DescriptorAllocator,
    pub layout_cache: DescriptorLayoutCache,

    pub global_descriptor_layout: vk::DescriptorSetLayout,
}

impl DescriptorManager {
    pub fn create(device: ConstPtr<Device>) -> DescriptorManager {
        let allocator = DescriptorAllocator::create(device);
        let mut layout_cache = DescriptorLayoutCache::create(device);
        let global_descriptor_layout = layout_cache.create_descriptor_layout_for_binding(&[
            layout_binding(0, vk::DescriptorType::UNIFORM_BUFFER, vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
        ]);
        DescriptorManager {
            allocator,
            layout_cache,
            global_descriptor_layout
        }
    }

    pub fn descriptor_builder(&mut self) -> DescriptorBuilder {
        DescriptorBuilder::begin(&mut self.layout_cache, &mut self.allocator)
    }
}