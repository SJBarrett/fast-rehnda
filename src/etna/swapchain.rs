use ash::extensions::khr;
use ash::vk;
use crate::etna::{ChosenSwapchainProps, QueueFamilyIndices};

pub struct Swapchain {
    device: *const ash::Device, // drop is already unsafe, so using a raw pointer
    swapchain: vk::SwapchainKHR,
    swapchain_fn: khr::Swapchain,
    image_format: vk::Format,
    extent: vk::Extent2D,
    images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
}

impl Swapchain {
    pub fn create(instance: &ash::Instance, device: &ash::Device, surface: &vk::SurfaceKHR, queue_family_indices: &QueueFamilyIndices, chosen_swapchain_props: &ChosenSwapchainProps) -> Swapchain {
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

        let queue_families_indices_unwrapped = [queue_family_indices.graphics_family.unwrap(), queue_family_indices.present_family.unwrap()];
        swapchain_creation_info = if queue_family_indices.graphics_family != queue_family_indices.present_family {
            swapchain_creation_info
                .image_sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(queue_families_indices_unwrapped.as_slice())
        } else {
            swapchain_creation_info
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        };
        let swapchain_fn = khr::Swapchain::new(instance, device);
        let swapchain = unsafe { swapchain_fn.create_swapchain(&swapchain_creation_info, None) }
            .expect("Failed to create the swapchain");

        let swapchain_images = unsafe { swapchain_fn.get_swapchain_images(swapchain) }
            .expect("Failed to get swapchain images");

        let image_views: Vec<vk::ImageView> = swapchain_images.iter().enumerate().map(|(index, swapchain_image)| {
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
        for image_view in &self.image_views {
            unsafe { (*self.device).destroy_image_view(*image_view, None) };
        }
        unsafe { self.swapchain_fn.destroy_swapchain(self.swapchain, None) }
    }
}