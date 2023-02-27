use ash::vk;
use crate::etna::pipelines::{DescriptorAllocationError, DescriptorAllocator, DescriptorLayoutCache};

pub struct DescriptorBuilder<'a> {
    layout_cache: &'a mut DescriptorLayoutCache,
    allocator: &'a mut DescriptorAllocator,
    bindings: Vec<vk::DescriptorSetLayoutBinding>,
    writes: Vec<vk::WriteDescriptorSet>,
}

impl<'a> DescriptorBuilder<'a> {
    pub fn begin(layout_cache: &'a mut DescriptorLayoutCache, allocator: &'a mut DescriptorAllocator) -> DescriptorBuilder<'a> {
        DescriptorBuilder {
            layout_cache,
            allocator,
            bindings: Vec::new(),
            writes: Vec::new(),
        }
    }

    pub fn build(mut self) -> Result<(vk::DescriptorSet, vk::DescriptorSetLayout), DescriptorAllocationError>{
        let layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(self.bindings.as_slice());
        let layout = self.layout_cache.create_descriptor_layout(&layout_info);
        let descriptor_set = self.allocator.allocate(&layout)?;

        self.writes.iter_mut().for_each(|write| write.dst_set = descriptor_set);

        unsafe { self.allocator.device.update_descriptor_sets(self.writes.as_slice(), &[]); }
        Ok((descriptor_set, layout))
    }

    pub fn bind_buffer(mut self, binding: u32, buffer_info: vk::DescriptorBufferInfoBuilder, descriptor_type: vk::DescriptorType, stage_flags: vk::ShaderStageFlags) -> Self {
        let new_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(binding)
            .descriptor_count(1)
            .descriptor_type(descriptor_type)
            .stage_flags(stage_flags)
            .build();
        self.bindings.push(new_binding);

        let new_write = vk::WriteDescriptorSet::builder()
            .dst_binding(binding)
            .descriptor_type(descriptor_type)
            .buffer_info(std::slice::from_ref(&buffer_info))
            .build();
        self.writes.push(new_write);
        self
    }

    pub fn bind_image(mut self, binding: u32, image_info: vk::DescriptorImageInfoBuilder, descriptor_type: vk::DescriptorType, stage_flags: vk::ShaderStageFlags) -> Self {
        let new_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(binding)
            .descriptor_count(1)
            .descriptor_type(descriptor_type)
            .stage_flags(stage_flags)
            .build();
        self.bindings.push(new_binding);

        let new_write = vk::WriteDescriptorSet::builder()
            .dst_binding(binding)
            .descriptor_type(descriptor_type)
            .image_info(std::slice::from_ref(&image_info))
            .build();
        self.writes.push(new_write);
        self
    }
}

pub fn buffer_binding<'a>(binding: u32, descriptor_type: vk::DescriptorType, stage_flags: vk::ShaderStageFlags) -> vk::DescriptorSetLayoutBindingBuilder<'a> {
    let new = vk::DescriptorSetLayoutBinding::builder()
        .binding(binding)
        .descriptor_count(1)
        .descriptor_type(descriptor_type)
        .stage_flags(stage_flags);
    new
}