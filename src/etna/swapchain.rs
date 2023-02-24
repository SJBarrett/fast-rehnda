use ash::extensions::khr;
use ash::vk;
use log::debug;

use crate::core::ConstPtr;
use crate::etna;
use crate::etna::{ChosenSwapchainProps, QueueFamilyIndices};

pub struct Swapchain {
    device: ConstPtr<etna::Device>,
    swapchain: vk::SwapchainKHR,
    swapchain_fn: khr::Swapchain,
    pub image_format: vk::Format,
    pub extent: vk::Extent2D,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
}

pub type SwapchainResult<T> = Result<T, SwapchainError>;

pub enum SwapchainError {
    RequiresRecreation,
}

impl Swapchain {
    pub fn acquire_next_image_and_get_index(&self, semaphore: vk::Semaphore) -> SwapchainResult<u32> {
        let acquire_result = unsafe { self.swapchain_fn.acquire_next_image(self.swapchain, u64::MAX, semaphore, vk::Fence::null()) };
        match acquire_result {
            Ok((image_index, _)) => Ok(image_index),
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Err(SwapchainError::RequiresRecreation),
            Err(unexpected_error) => panic!("Unexpected error acquiring next image: {}", unexpected_error),
        }
    }

    pub fn present(&self, image_index: u32, signal_semaphores: &[vk::Semaphore]) -> SwapchainResult<()> {
        let swapchains = &[self.swapchain];
        let image_indices = &[image_index];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(signal_semaphores)
            .swapchains(swapchains)
            .image_indices(image_indices);
        let present_result = unsafe { self.swapchain_fn.queue_present(self.device.present_queue, &present_info) };
        match present_result {
            Ok(_) => Ok(()),
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) | Err(vk::Result::SUBOPTIMAL_KHR) => Err(SwapchainError::RequiresRecreation),
            Err(unexpected_error) => panic!("Unexpected error presenting swapchain: {}", unexpected_error),
        }
    }

    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.extent.width as f32 / self.extent.height as f32
    }
}

// intialisation functionality
impl Swapchain {
    pub fn recreate(&mut self, surface: &vk::SurfaceKHR, queue_family_indices: &QueueFamilyIndices, chosen_swapchain_props: ChosenSwapchainProps) {
        debug!("Recreating swapchain");
        unsafe { self.device.device_wait_idle() }
            .expect("Failed to wait for device idle when recreating swapchain");
        self.destroy_resources();
        let image_format = chosen_swapchain_props.surface_format.format;
        let extent = chosen_swapchain_props.extent;
        let (swapchain, images, image_views) = Self::create_swapchain_resources(&self.device, &self.swapchain_fn, surface, queue_family_indices, chosen_swapchain_props);
        self.image_format = image_format;
        self.extent = extent;
        self.swapchain = swapchain;
        self.images = images;
        self.image_views = image_views;
    }
    pub fn create(instance: &ash::Instance, device: ConstPtr<etna::Device>, surface: &vk::SurfaceKHR, queue_family_indices: &QueueFamilyIndices, chosen_swapchain_props: ChosenSwapchainProps) -> Swapchain {
        let swapchain_fn = khr::Swapchain::new(instance, &device);

        let image_format = chosen_swapchain_props.surface_format.format;
        let extent = chosen_swapchain_props.extent;
        let (swapchain, images, image_views) = Self::create_swapchain_resources(&device, &swapchain_fn, surface, queue_family_indices, chosen_swapchain_props);

        Swapchain {
            device,
            swapchain_fn,
            swapchain,
            images,
            image_views,
            image_format,
            extent,
        }
    }

    fn create_swapchain_resources(device: &etna::Device, swapchain_fn: &khr::Swapchain, surface: &vk::SurfaceKHR, queue_family_indices: &QueueFamilyIndices, chosen_swapchain_props: ChosenSwapchainProps) -> (vk::SwapchainKHR, Vec<vk::Image>, Vec<vk::ImageView>){
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

        (swapchain, swapchain_images, image_views)
    }

    fn destroy_resources(&mut self) {
        debug!("Destroying swapchain");
        unsafe {
            for image_view in &self.image_views {
                self.device.destroy_image_view(*image_view, None);
            }
            self.swapchain_fn.destroy_swapchain(self.swapchain, None);
            self.image_views.clear();
            self.images.clear();
            self.swapchain = vk::SwapchainKHR::null();
        }
    }
}


impl Drop for Swapchain {
    fn drop(&mut self) {
        self.destroy_resources();
    }
}