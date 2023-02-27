use std::hash::{Hash};
use ahash::AHashMap;
use ash::vk;
use crate::core::ConstPtr;
use crate::etna::Device;

pub struct DescriptorLayoutCache {
    device: ConstPtr<Device>,
    layout_cache: AHashMap<DescriptorLayoutInfo, vk::DescriptorSetLayout>,
}

impl DescriptorLayoutCache {
    pub fn create(device: ConstPtr<Device>) -> DescriptorLayoutCache {
        DescriptorLayoutCache {
            device,
            layout_cache: AHashMap::new(),
        }
    }

    pub fn create_descriptor_layout(&mut self, create_info: &vk::DescriptorSetLayoutCreateInfo) -> vk::DescriptorSetLayout {
        let mut bindings: Vec<EtnaDescriptorSetLayoutBinding> = unsafe { std::slice::from_raw_parts(create_info.p_bindings, create_info.binding_count as usize) }
            .iter()
            .map(|binding| EtnaDescriptorSetLayoutBinding::from(*binding))
            .collect();
        // ensure bindings are in strictly increasing order
        bindings.sort_by_key(|k| k.binding);
        let cache_key = DescriptorLayoutInfo { bindings };

        if let Some(cached_value) = self.layout_cache.get(&cache_key) {
            *cached_value
        } else {
            // create new layout and add to the cache
            let new_layout = unsafe { self.device.create_descriptor_set_layout(create_info, None) }
                .expect("Failed to create descriptor set layout");
            self.layout_cache.insert(cache_key, new_layout);
            new_layout
        }
    }

    pub fn clear_cache(&mut self) {
        unsafe {
            for (_, set_layout) in &self.layout_cache {
                self.device.destroy_descriptor_set_layout(*set_layout, None);
            }
        }
        self.layout_cache.clear();
    }
}

impl Drop for DescriptorLayoutCache {
    fn drop(&mut self) {
        self.clear_cache()
    }
}

#[derive(Eq, PartialEq, Hash)]
struct DescriptorLayoutInfo {
    bindings: Vec<EtnaDescriptorSetLayoutBinding>
}

#[derive(Eq, PartialEq, Hash)]
struct EtnaDescriptorSetLayoutBinding {
    pub binding: u32,
    pub descriptor_type: vk::DescriptorType,
    pub descriptor_count: u32,
    pub stage_flags: vk::ShaderStageFlags,
    pub p_immutable_samplers: *const vk::Sampler,
}

impl From<vk::DescriptorSetLayoutBinding> for EtnaDescriptorSetLayoutBinding {
    fn from(value: vk::DescriptorSetLayoutBinding) -> Self {
        EtnaDescriptorSetLayoutBinding {
            binding: value.binding,
            descriptor_type: value.descriptor_type,
            descriptor_count: value.descriptor_count,
            stage_flags: value.stage_flags,
            p_immutable_samplers: value.p_immutable_samplers,
        }
    }
}