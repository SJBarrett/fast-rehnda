use std::ffi::c_char;
use std::ptr;

use ash::vk;
use ash::vk::PhysicalDevice;

use crate::rvk::{Instance, VALIDATION_LAYERS};

pub struct Device {
    device: ash::Device,
    pub graphics_queue: vk::Queue,
}

impl Device {
    pub fn create(instance: &Instance, physical_device: PhysicalDevice) -> Device {
        let indices = instance.find_queue_families(physical_device);
        let graphics_family_queue_index = indices.graphics_family.expect("Graphics family must be available");
        let queue_create_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(graphics_family_queue_index)
            .queue_priorities(&[1.0]);

        let validation_layer_names = VALIDATION_LAYERS.map(|layer| layer.as_ptr() as *const c_char);
        let device_create_info = vk::DeviceCreateInfo {
            p_queue_create_infos: &queue_create_info.build(),
            queue_create_info_count: 1,
            enabled_layer_count: VALIDATION_LAYERS.len() as u32,
            pp_enabled_layer_names: if VALIDATION_LAYERS.is_empty() {
                ptr::null()
            } else {
                validation_layer_names.as_ptr()
            },
            ..Default::default()
        };

        // TODO fix warn message logged on creating device that prints
        // "vkGetPhysicalDeviceProperties2KHR: Emulation found unrecognized structure type in pProperties->pNext - this struct will be ignored"
        let device = unsafe { (*instance).create_device(physical_device, &device_create_info, None) }
            .expect("Failed to create device");
        let graphics_queue = unsafe { device.get_device_queue(graphics_family_queue_index, 0) };
        Device {
            device,
            graphics_queue,
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe { self.device.destroy_device(None); }
    }
}