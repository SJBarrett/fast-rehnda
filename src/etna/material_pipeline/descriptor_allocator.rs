use ash::vk;

use crate::core::ConstPtr;
use crate::etna::Device;

// this abstraction is an implementation of the abstraction described here -> https://vkguide.dev/docs/extra-chapter/abstracting_descriptors/

pub const POOL_SIZES: [(vk::DescriptorType, f32); 11] = [
    (vk::DescriptorType::SAMPLER, 0.5),
    (vk::DescriptorType::COMBINED_IMAGE_SAMPLER, 4.0),
    (vk::DescriptorType::SAMPLED_IMAGE, 4.0),
    (vk::DescriptorType::STORAGE_IMAGE, 1.0),
    (vk::DescriptorType::UNIFORM_TEXEL_BUFFER, 1.0),
    (vk::DescriptorType::STORAGE_TEXEL_BUFFER, 1.0),
    (vk::DescriptorType::UNIFORM_BUFFER, 2.0),
    (vk::DescriptorType::STORAGE_BUFFER, 2.0),
    (vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC, 1.0),
    (vk::DescriptorType::STORAGE_BUFFER_DYNAMIC, 1.0),
    (vk::DescriptorType::INPUT_ATTACHMENT, 0.5),
];

pub struct DescriptorAllocator {
    pub device: ConstPtr<Device>,
    current_pool: Option<vk::DescriptorPool>,
    descriptor_sizes: Vec<(vk::DescriptorType, f32)>,
    used_pools: Vec<vk::DescriptorPool>,
    free_pools: Vec<vk::DescriptorPool>,
}

#[derive(Debug)]
pub enum DescriptorAllocationError {
    UnrecoverableError
}

impl DescriptorAllocator {
    pub fn allocate(&mut self, layout: &vk::DescriptorSetLayout) -> Result<vk::DescriptorSet, DescriptorAllocationError> {
        let current_pool = {
            if let Some(current_pool) = self.current_pool {
                current_pool
            } else {
                self.allocate_new_current_pool()
            }
        };

        let alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .set_layouts(std::slice::from_ref(layout))
            .descriptor_pool(current_pool);

        let result = unsafe { self.device.allocate_descriptor_sets(&alloc_info) };

        match result {
            Ok(allocated_sets) => Ok(allocated_sets[0]),
            Err(vk::Result::ERROR_FRAGMENTED_POOL) | Err(vk::Result::ERROR_OUT_OF_POOL_MEMORY) => {
                // allocate a new pool and try again with the new pool
                let new_pool = self.allocate_new_current_pool();
                let new_alloc_info = vk::DescriptorSetAllocateInfo::builder()
                    .set_layouts(std::slice::from_ref(layout))
                    .descriptor_pool(new_pool);
                if let Ok(new_allocated_sets) = unsafe { self.device.allocate_descriptor_sets(&new_alloc_info) } {
                    Ok(new_allocated_sets[0])
                } else {
                    Err(DescriptorAllocationError::UnrecoverableError)
                }
            }
            _ => Err(DescriptorAllocationError::UnrecoverableError),
        }
    }

    pub fn create(device: ConstPtr<Device>) -> DescriptorAllocator {
        DescriptorAllocator {
            device,
            current_pool: None,
            descriptor_sizes: Vec::from(POOL_SIZES),
            used_pools: Vec::new(),
            free_pools: Vec::new(),
        }
    }

    pub fn reset_pools(&mut self) {
        for used_pool in self.used_pools.iter() {
            unsafe { self.device.reset_descriptor_pool(*used_pool, vk::DescriptorPoolResetFlags::empty()) }
                .expect("Failed to reset descriptor pool");
        }
        self.used_pools.clear();
        self.current_pool = None;
    }

    fn allocate_new_current_pool(&mut self) -> vk::DescriptorPool {
        let new_pool = self.grab_pool();
        self.current_pool = Some(new_pool);
        self.used_pools.push(new_pool);
        new_pool
    }

    fn grab_pool(&mut self) -> vk::DescriptorPool {
        if let Some(descriptor_pool) = self.free_pools.pop() {
            descriptor_pool
        } else {
            create_pool(&self.device, self.descriptor_sizes.as_slice(), 1000, vk::DescriptorPoolCreateFlags::empty())
        }
    }
}

fn create_pool(device: &Device, pool_sizes: &[(vk::DescriptorType, f32)], count: u32, create_flags: vk::DescriptorPoolCreateFlags) -> vk::DescriptorPool {
    let sizes: Vec<vk::DescriptorPoolSize> = pool_sizes.iter().map(|(descriptor_type, size)| {
        vk::DescriptorPoolSize {
            ty: *descriptor_type,
            descriptor_count: (size * count as f32) as u32,
        }
    })
        .collect();
    let pool_ci = vk::DescriptorPoolCreateInfo::builder()
        .flags(create_flags)
        .pool_sizes(sizes.as_slice())
        .max_sets(count);
    unsafe { device.create_descriptor_pool(&pool_ci, None) }
        .expect("Failed to create descriptor pool")
}

impl Drop for DescriptorAllocator {
    fn drop(&mut self) {
        unsafe {
            for pool in self.free_pools.iter().chain(self.used_pools.iter()) {
                self.device.destroy_descriptor_pool(*pool, None);
            }
        }
    }
}
