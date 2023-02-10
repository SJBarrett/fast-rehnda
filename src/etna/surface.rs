use std::ops::Deref;
use ash::{Entry, Instance, vk};
use ash::prelude::VkResult;
use ash::vk::PhysicalDevice;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

pub struct Surface {
    surface: vk::SurfaceKHR,
    surface_fn: ash::extensions::khr::Surface,
}

impl Deref for Surface {
    type Target = vk::SurfaceKHR;

    fn deref(&self) -> &Self::Target {
        &self.surface
    }
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


    pub fn query_swapchain_support_details(&self, physical_device: PhysicalDevice) -> SwapchainSupportDetails {
        let capabilities = unsafe { self.surface_fn.get_physical_device_surface_capabilities(physical_device, self.surface) }
            .expect("Failed to get physical device surface capabilities");
        let formats = unsafe { self.surface_fn.get_physical_device_surface_formats(physical_device, self.surface) }
            .expect("Failed to get physical device surface formats");
        let present_modes = unsafe { self.surface_fn.get_physical_device_surface_present_modes(physical_device, self.surface) }
            .expect("Failed to get physical device present modes");
        SwapchainSupportDetails {
            capabilities,
            formats,
            present_modes,
        }
    }

    pub fn query_best_swapchain_creation_details(&self, window: &winit::window::Window, physical_device: PhysicalDevice) -> ChosenSwapchainProps {
        let support_details = self.query_swapchain_support_details(physical_device);
        ChosenSwapchainProps {
            capabilities: support_details.capabilities,
            surface_format: Self::choose_surface_format(&support_details.formats),
            present_mode: Self::choose_present_mode(&support_details.present_modes),
            extent: Self::choose_swapchain_extent(window, &support_details.capabilities),
        }
    }

    fn choose_surface_format(available_formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
        available_formats.iter()
            .find(|&&available_format|
                available_format.format == vk::Format::B8G8R8A8_SRGB && available_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .map_or(available_formats[0], |chosen_format| *chosen_format)
    }

    fn choose_present_mode(available_present_modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
        available_present_modes.iter()
            .find(|&&available_present_mode| available_present_mode == vk::PresentModeKHR::MAILBOX)
            .map_or(vk::PresentModeKHR::FIFO, |chosen_present_mode| *chosen_present_mode)
    }

    fn choose_swapchain_extent(window: &winit::window::Window, surface_capabilities: &vk::SurfaceCapabilitiesKHR) -> vk::Extent2D {
        if surface_capabilities.current_extent.width != u32::MAX {
            return surface_capabilities.current_extent;
        }
        let window_size = window.inner_size();

        let chosen_width = window_size.width.clamp(surface_capabilities.min_image_extent.width, surface_capabilities.max_image_extent.width);
        let chosen_height = window_size.height.clamp(surface_capabilities.min_image_extent.height, surface_capabilities.max_image_extent.height);

        vk::Extent2D {
            width: chosen_width,
            height: chosen_height,
        }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe { self.surface_fn.destroy_surface(self.surface, None); }
    }
}

pub struct SwapchainSupportDetails {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

pub struct ChosenSwapchainProps {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub surface_format: vk::SurfaceFormatKHR,
    pub present_mode: vk::PresentModeKHR,
    pub extent: vk::Extent2D,
}

