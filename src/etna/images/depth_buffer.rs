use std::sync::Arc;
use ash::vk;
use ash::vk::Extent2D;
use crate::etna::{CommandPool, Device, Image, ImageCreateInfo, PhysicalDevice};
use crate::etna::image_transitions::{transition_image_layout, TransitionProps};

pub struct DepthBuffer {
    pub image: Image,
    pub format: vk::Format,
}

impl Drop for DepthBuffer {
    fn drop(&mut self) {
    }
}

impl DepthBuffer {
    pub fn create(device: Arc<Device>, physical_device: &PhysicalDevice, command_pool: &CommandPool, extent: Extent2D) -> DepthBuffer {
        let candidate_formats = [vk::Format::D32_SFLOAT, vk::Format::D32_SFLOAT_S8_UINT, vk::Format::D24_UNORM_S8_UINT];
        let depth_format = physical_device.find_supported_format(&candidate_formats, vk::ImageTiling::OPTIMAL, vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
            .expect("Failed to find supported format for depth buffer");
        let image = Image::create_image(device.clone(), physical_device, &ImageCreateInfo {
            width: extent.width,
            height: extent.height,
            mip_levels: 1,
            format: depth_format,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            memory_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
            image_aspect_flags: vk::ImageAspectFlags::DEPTH,
        });

        let one_time_command_buffer = command_pool.one_time_command_buffer();
        transition_image_layout(&device, &one_time_command_buffer, image.vk_image, &TransitionProps {
            old_layout: vk::ImageLayout::UNDEFINED,
            src_access_mask: vk::AccessFlags2::NONE,
            src_stage_mask: vk::PipelineStageFlags2::TOP_OF_PIPE,
            new_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            dst_access_mask: vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_READ | vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
            dst_stage_mask: vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS,
            aspect_mask: if Self::format_has_stencil(depth_format) {
                vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
            } else {
                vk::ImageAspectFlags::DEPTH
            },
            base_mip_level: 0,
            level_count: 1,
        });

        DepthBuffer {
            image,
            format: depth_format,
        }
    }

    fn format_has_stencil(format: vk::Format) -> bool {
        format == vk::Format::D32_SFLOAT_S8_UINT || format == vk::Format::D24_UNORM_S8_UINT
    }
}