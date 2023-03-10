use ash::extensions::khr;
use ash::vk;
use bevy_ecs::prelude::*;
use log::debug;

use crate::etna;
use crate::etna::{ChosenSwapchainProps, CommandPool, DepthBuffer, Image, ImageCreateInfo, PhysicalDevice, PhysicalDeviceRes, QueueFamilyIndices, Surface};
use crate::rehnda_core::ConstPtr;

#[derive(Resource)]
pub struct Swapchain {
    device: ConstPtr<etna::Device>,
    swapchain: vk::SwapchainKHR,
    swapchain_fn: khr::Swapchain,
    pub image_format: vk::Format,
    pub extent: vk::Extent2D,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
    pub depth_buffer: DepthBuffer,
    pub color_image: Image,
    pub msaa_enabled: bool,

    pub needs_recreation: bool,
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
    pub fn recreate(&mut self, physical_device: &PhysicalDevice, surface: &vk::SurfaceKHR, command_pool: &CommandPool, queue_family_indices: &QueueFamilyIndices, chosen_swapchain_props: ChosenSwapchainProps) {
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
        self.depth_buffer = DepthBuffer::create(self.device, physical_device, command_pool, extent);
        self.color_image = Image::create_image(self.device, &multisampling_color_image_create_info(physical_device, extent, image_format));
    }
    pub fn create(instance: &ash::Instance, device: ConstPtr<etna::Device>, physical_device: &PhysicalDevice, surface: &vk::SurfaceKHR, command_pool: &CommandPool, queue_family_indices: &QueueFamilyIndices, chosen_swapchain_props: ChosenSwapchainProps) -> Swapchain {
        let swapchain_fn = khr::Swapchain::new(instance, &device);

        let image_format = chosen_swapchain_props.surface_format.format;
        let extent = chosen_swapchain_props.extent;
        let (swapchain, images, image_views) = Self::create_swapchain_resources(&device, &swapchain_fn, surface, queue_family_indices, chosen_swapchain_props);
        let depth_buffer = DepthBuffer::create(device, physical_device, command_pool, extent);
        let color_image = Image::create_image(device, &multisampling_color_image_create_info(physical_device, extent, image_format));
        Swapchain {
            device,
            swapchain_fn,
            swapchain,
            images,
            image_views,
            image_format,
            extent,
            depth_buffer,
            color_image,
            msaa_enabled: physical_device.graphics_settings.is_msaa_enabled(),
            needs_recreation: false,
        }
    }

    fn create_swapchain_resources(device: &etna::Device, swapchain_fn: &khr::Swapchain, surface: &vk::SurfaceKHR, queue_family_indices: &QueueFamilyIndices, chosen_swapchain_props: ChosenSwapchainProps) -> (vk::SwapchainKHR, Vec<vk::Image>, Vec<vk::ImageView>) {
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


fn multisampling_color_image_create_info(physical_device: &PhysicalDevice, extent: vk::Extent2D, format: vk::Format) -> ImageCreateInfo {
    ImageCreateInfo {
        width: extent.width,
        height: extent.height,
        format,
        tiling: vk::ImageTiling::OPTIMAL,
        usage: vk::ImageUsageFlags::TRANSIENT_ATTACHMENT | vk::ImageUsageFlags::COLOR_ATTACHMENT,
        mip_levels: 1,
        memory_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
        image_aspect_flags: vk::ImageAspectFlags::COLOR,
        num_samples: physical_device.graphics_settings.msaa_samples.to_sample_count_flags(),
    }
}

pub mod swapchain_systems {
    use bevy_ecs::prelude::*;

    use crate::ecs_engine::EtnaWindow;
    use crate::etna::{CommandPool, PhysicalDeviceRes, Surface, Swapchain};
    use crate::scene::Camera;

    pub fn swap_chain_recreation_system(mut swapchain: ResMut<Swapchain>, physical_device: PhysicalDeviceRes, surface: Res<Surface>, command_pool: Res<CommandPool>, window: Res<EtnaWindow>, mut camera: ResMut<Camera>) {
        swapchain.recreate(&physical_device, &surface, &command_pool, &physical_device.queue_families(), surface.query_best_swapchain_creation_details(window.winit_window.inner_size(), physical_device.handle()));
        camera.update_aspect_ratio(swapchain.aspect_ratio());
    }

    pub fn swap_chain_needs_recreation(swapchain: Res<Swapchain>) -> bool {
        swapchain.needs_recreation
    }
}