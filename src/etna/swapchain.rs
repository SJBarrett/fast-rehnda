use std::sync::Arc;
use ash::extensions::khr;
use ash::vk;
use crate::etna;
use crate::etna::{ChosenSwapchainProps, QueueFamilyIndices};

pub struct Swapchain {
    device: Arc<etna::Device>,
    swapchain: vk::SwapchainKHR,
    swapchain_fn: khr::Swapchain,
    image_format: vk::Format,
    extent: vk::Extent2D,
    images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
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

    pub fn render_pass(&self) -> vk::RenderPass {
        self.render_pass
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

        let render_pass = Self::create_render_pass(&device, swapchain_creation_info.image_format);

        Swapchain {
            device,
            swapchain,
            swapchain_fn,
            image_format: chosen_swapchain_props.surface_format.format,
            extent: chosen_swapchain_props.extent,
            images: swapchain_images,
            image_views,
            render_pass,
        }
    }

    fn create_render_pass(device: &etna::Device, format: vk::Format) -> vk::RenderPass {
        // render pass creation
        let color_attachment = vk::AttachmentDescription::builder()
            .format(format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

        let color_attachment_ref = vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let color_attachment_refs = &[color_attachment_ref.build()];
        let subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(color_attachment_refs);

        let dependency = vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

        let attachments = &[color_attachment.build()];
        let subpasses = &[subpass.build()];
        let dependencies = &[dependency.build()];
        let render_pass_ci = vk::RenderPassCreateInfo::builder()
            .attachments(attachments)
            .subpasses(subpasses)
            .dependencies(dependencies);

        unsafe { device.create_render_pass(&render_pass_ci, None) }
            .expect("Failed to create render pass")
    }
}


impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_render_pass(self.render_pass, None);
            for image_view in &self.image_views {
                self.device.destroy_image_view(*image_view, None);
            }
            self.swapchain_fn.destroy_swapchain(self.swapchain, None);
        }
    }
}