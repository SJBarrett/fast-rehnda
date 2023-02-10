use ash::{Entry, Instance, vk};
use ash::prelude::VkResult;
use ash::vk::PhysicalDevice;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

pub struct Surface {
    surface: vk::SurfaceKHR,
    surface_fn: ash::extensions::khr::Surface,
}

impl Surface {
    pub fn new(entry: &Entry, instance: &Instance, raw_display_handle: RawDisplayHandle, raw_window_handle: RawWindowHandle) -> Result<Surface, vk::Result> {
        let surface = unsafe { ash_window::create_surface(entry, instance, raw_display_handle, raw_window_handle, None) }?;
        let surface_fn = ash::extensions::khr::Surface::new(entry, instance);
        Ok(Surface {
            surface,
            surface_fn,
        })
    }

    pub fn get_physical_device_surface_support(&self, physical_device: PhysicalDevice, queue_family_index: u32) -> VkResult<bool> {
        unsafe { self.surface_fn.get_physical_device_surface_support(physical_device, queue_family_index, self.surface) }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe { self.surface_fn.destroy_surface(self.surface, None); }
    }
}
