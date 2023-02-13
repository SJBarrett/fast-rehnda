use std::sync::Arc;
use ash::extensions::khr;
use ash::vk;
use crate::etna;
use crate::etna::{ChosenSwapchainProps, QueueFamilyIndices};

pub struct Swapchain {
    device: Arc<etna::Device>,
    swapchain: vk::SwapchainKHR,
    swapchain_fn: khr::Swapchain,
    pub image_format: vk::Format,
    pub extent: vk::Extent2D,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
}

impl Swapchain {
    pub fn acquire_next_image_and_get_index(&self, semaphore: vk::Semaphore) -> u32 {
        unsafe { self.swapchain_fn.acquire_next_image(self.swapchain, u64::MAX, semaphore, vk::Fence::null()) }
            .expect("Failed to acquire next swapchain image").0
    }

    pub fn present(&self, image_index: u32, signal_semaphores: &[vk::Semaphore]) {
        let swapchains = &[self.swapchain];
        let image_indices = &[image_index];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(signal_semaphores)
            .swapchains(swapchains)
            .image_indices(image_indices);
        unsafe { self.swapchain_fn.queue_present(self.device.present_queue, &present_info) }
            .expect("Failed to present to swapchain");
    }

    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    pub fn image_views(&self) -> &Vec<vk::ImageView> {
        &self.image_views
    }
}

// intialisation functionality
impl Swapchain {
    pub fn create(instance: &ash::Instance, device: Arc<etna::Device>, surface: &vk::SurfaceKHR, queue_family_indices: &QueueFamilyIndices, chosen_swapchain_props: &ChosenSwapchainProps) -> Swapchain {
        // request one more than the min to avoid waiting on the driver
        let mut image_count = chosen_swapchain_props.capabilities.min_image_count + 1;
        if chosen_swapchain_props.capabilities.max_image_count > 0 && image_count > chosen_swapchain_props.capabilities.max_image_count {
            image_count = chosen_swapchain_props.capabilities.max_image_count;
        }

        let mut swapchain_creation_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(*surface)
            .min_image_count(image_count)
            .image_format(chosen_swapchain_props.surface_format.format)
            .image_color_space(chosen_swapchain_props.surface_format.color_space)
            .image_extent(chosen_swapchain_props.extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(chosen_swapchain_props.capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(chosen_swapchain_props.present_mode)
            .clipped(true)
            .old_swapchain(vk::SwapchainKHR::null()) // TODO populate with old swapchain reference on re-creation
            ;

        let queue_families_indices_unwrapped = [queue_family_indices.graphics_family, queue_family_indices.present_family];
        swapchain_creation_info = if queue_family_indices.graphics_family != queue_family_indices.present_family {
            swapchain_creation_info
                .image_sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(queue_families_indices_unwrapped.as_slice())
        } else {
            swapchain_creation_info
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        };
        let swapchain_fn = khr::Swapchain::new(instance, &device);
        let swapchain = unsafe { swapchain_fn.create_swapchain(&swapchain_creation_info, None) }
            .expect("Failed to create the swapchain");

        let swapchain_images = unsafe { swapchain_fn.get_swapchain_images(swapchain) }
            .expect("Failed to get swapchain images");

        let image_views: Vec<vk::ImageView> = swapchain_images.iter().map(|swapchain_image| {
            let image_view_ci = vk::ImageViewCreateInfo::builder()
                .image(*swapchain_image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(chosen_swapchain_props.surface_format.format)
                .components(vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                })
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });
            unsafe { device.create_image_view(&image_view_ci, None) }
                .expect("Failed to create image view")
        }).collect();

        Swapchain {
            device,
            swapchain,
            swapchain_fn,
            image_format: chosen_swapchain_props.surface_format.format,
            extent: chosen_swapchain_props.extent,
            images: swapchain_images,
            image_views,
        }
    }
}


impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            for image_view in &self.image_views {
                self.device.destroy_image_view(*image_view, None);
            }
            self.swapchain_fn.destroy_swapchain(self.swapchain, None);
        }
    }
}