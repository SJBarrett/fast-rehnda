use std::cell::UnsafeCell;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::os::raw::c_char;

use ash::vk;
use bevy_ecs::prelude::Res;
use bevy_ecs::system::Resource;
use gpu_allocator::AllocatorDebugSettings;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, Allocator, AllocatorCreateDesc};

use crate::etna;
use crate::etna::{DEVICE_EXTENSIONS, VALIDATION_LAYERS};
use crate::rehnda_core::LongLivedObject;

pub type DeviceRes<'w> = Res<'w, LongLivedObject<Device>>;

#[derive(Resource)]
pub struct Device {
    device: ash::Device,
    pub allocator: ManuallyDrop<UnsafeCell<Allocator>>,
    pub enabled_features: vk::PhysicalDeviceFeatures,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
}

impl Deref for Device {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl Device {
    pub fn create(instance: &etna::Instance, surface: &etna::Surface, physical_device: &etna::PhysicalDevice) -> Device {
        let queue_indices = instance.find_queue_families(surface, physical_device.handle());
        let graphics_family_queue_index = queue_indices.graphics_family.expect("Graphics family must be available");
        let present_family_queue_index = queue_indices.present_family.expect("Present family must be available");

        use std::collections::HashSet;
        let unique_queue_families = HashSet::from([
            graphics_family_queue_index,
            present_family_queue_index,
        ]);
        let queue_create_infos: Vec<vk::DeviceQueueCreateInfo> = unique_queue_families.iter().map(|unique_queue_family_index|  vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(*unique_queue_family_index)
            .queue_priorities(&[1.0]).build())
            .collect();
        let validation_layer_names = VALIDATION_LAYERS.map(|layer| layer.as_ptr() as *const c_char);
        let device_extension_names = DEVICE_EXTENSIONS.map(|extension| extension.as_ptr() as *const c_char);
        // enable dynamic rendering
        let mut dynamic_rendering_feature = vk::PhysicalDeviceDynamicRenderingFeatures::builder()
            .dynamic_rendering(true)
            .build();
        let mut synchronization_2_feature = vk::PhysicalDeviceSynchronization2Features::builder()
            .synchronization2(true)
            .build();
        let mut buffer_device_address_feature = vk::PhysicalDeviceBufferDeviceAddressFeatures::builder()
            .buffer_device_address(true)
            .build();
        let physical_device_features = vk::PhysicalDeviceFeatures::builder()
            .sampler_anisotropy(physical_device.supported_features.sampler_anisotropy == vk::TRUE)
            .sample_rate_shading(physical_device.graphics_settings.sample_rate_shading_enabled);
        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(queue_create_infos.as_slice())
            .enabled_layer_names(validation_layer_names.as_slice())
            .enabled_extension_names(device_extension_names.as_slice())
            .enabled_features(&physical_device_features)
            .push_next(&mut dynamic_rendering_feature)
            .push_next(&mut synchronization_2_feature)
            .push_next(&mut buffer_device_address_feature);


        let device = unsafe { (*instance).create_device(physical_device.handle(), &device_create_info, None) }
            .expect("Failed to create device");
        let graphics_queue = unsafe { device.get_device_queue(graphics_family_queue_index, 0) };
        let present_queue = unsafe { device.get_device_queue(present_family_queue_index, 0) };

        let debug = AllocatorDebugSettings {
            log_memory_information: false,
            log_leaks_on_shutdown: true,
            store_stack_traces: false,
            log_allocations: false,
            log_frees: false,
            log_stack_traces: false,
        };
        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: instance.ash_handle(),
            device: device.clone(),
            physical_device: physical_device.handle(),
            debug_settings: debug,
            buffer_device_address: true,
        })
            .expect("Failed to create allocator");

        Device {
            device,
            enabled_features: physical_device_features.build(),
            graphics_queue,
            present_queue,
            allocator: ManuallyDrop::new(UnsafeCell::new(allocator)),
        }
    }

    pub fn allocate(&self, allocation_desc: &AllocationCreateDesc) -> gpu_allocator::Result<Allocation> {
         unsafe { (*self.allocator.get()).allocate(allocation_desc) }
    }

    pub fn free_allocation(&self, allocation: Allocation) {
        unsafe { (*self.allocator.get()).free(allocation) }
            .expect("Failed to free memory allocation")
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.allocator);
            self.device.destroy_device(None);
        }
    }
}

unsafe impl Send for Device {}
unsafe impl Sync for Device {}