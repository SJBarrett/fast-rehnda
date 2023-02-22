use std::sync::Arc;

use ash::vk;

use crate::etna::{Device, PhysicalDevice};

pub struct Image {
    device: Arc<Device>,
    pub vk_image: vk::Image,
    pub device_memory: vk::DeviceMemory,
    pub image_view: vk::ImageView,
    pub mip_levels: u32,
    pub format: vk::Format,
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image_view(self.image_view, None);
            self.device.destroy_image(self.vk_image, None);
            self.device.free_memory(self.device_memory, None);
        }
    }
}

pub struct ImageCreateInfo {
    pub width: u32,
    pub height: u32,
    pub format: vk::Format,
    pub tiling: vk::ImageTiling,
    pub usage: vk::ImageUsageFlags,
    pub mip_levels: u32,
    pub memory_properties: vk::MemoryPropertyFlags,
    pub image_aspect_flags: vk::ImageAspectFlags,
    pub num_samples: vk::SampleCountFlags,
}

impl Image {
    pub fn create_image(device: Arc<Device>, physical_device: &PhysicalDevice, create_info: &ImageCreateInfo) -> Image {
        let image_ci = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                width: create_info.width,
                height: create_info.height,
                depth: 1,
            })
            .mip_levels(create_info.mip_levels)
            .array_layers(1)
            .format(create_info.format)
            .tiling(create_info.tiling)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(create_info.usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(create_info.num_samples)
            ;

        let image = unsafe { device.create_image(&image_ci, None) }
            .expect("Failed to create image for texture");

        let memory_requirements = unsafe { device.get_image_memory_requirements(image) };
        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(physical_device.find_memory_type(memory_requirements.memory_type_bits, create_info.memory_properties));
        let device_memory = unsafe { device.allocate_memory(&alloc_info, None) }
            .expect("Failed to allocate memory for texture");
        unsafe { device.bind_image_memory(image, device_memory, 0) }
            .expect("Failed to bind image memory for texture");

        let view_ci = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(create_info.format)
            .subresource_range(vk::ImageSubresourceRange::builder()
                .aspect_mask(create_info.image_aspect_flags)
                .base_mip_level(0)
                .level_count(create_info.mip_levels)
                .base_array_layer(0)
                .layer_count(1)
                .build()
            );
        let image_view = unsafe { device.create_image_view(&view_ci, None) }
            .expect("Failed to create image view");

        Image {
            device,
            vk_image: image,
            image_view,
            device_memory,
            mip_levels: create_info.mip_levels,
            format: create_info.format,
        }
    }
}