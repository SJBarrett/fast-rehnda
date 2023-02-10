use std::os::raw::c_char;

use ash::vk;
use ash::vk::PhysicalDevice;
use crate::rvk;

use crate::rvk::{VALIDATION_LAYERS};

pub struct Device {
    device: ash::Device,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
}

impl Device {
    pub fn create(instance: &rvk::Instance, surface: &rvk::Surface, physical_device: PhysicalDevice) -> Device {
        let queue_indices = instance.find_queue_families(surface, physical_device);
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
        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(queue_create_infos.as_slice())
            .enabled_layer_names(validation_layer_names.as_slice());


        let device = unsafe { (*instance).create_device(physical_device, &device_create_info, None) }
            .expect("Failed to create device");
        let graphics_queue = unsafe { device.get_device_queue(graphics_family_queue_index, 0) };
        let present_queue = unsafe { device.get_device_queue(present_family_queue_index, 0) };
        Device {
            device,
            graphics_queue,
            present_queue,
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe { self.device.destroy_device(None); }
    }
}