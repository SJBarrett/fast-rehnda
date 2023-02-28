use std::mem::ManuallyDrop;

use ash::vk;
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme};

use crate::etna::Device;
use crate::rehnda_core::ConstPtr;

pub struct Image {
    device: ConstPtr<Device>,
    pub vk_image: vk::Image,
    pub allocation: ManuallyDrop<Allocation>,
    pub image_view: vk::ImageView,
    pub mip_levels: u32,
    pub format: vk::Format,
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image_view(self.image_view, None);
            self.device.destroy_image(self.vk_image, None);
            self.device.free_allocation(ManuallyDrop::take(&mut self.allocation));
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
    pub fn create_image(device: ConstPtr<Device>, create_info: &ImageCreateInfo) -> Image {
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
        let allocation = device.allocate(&AllocationCreateDesc {
            name: "Image memory",
            requirements: memory_requirements,
            location: MemoryLocation::GpuOnly,
            linear: false,
            allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        }).expect("Failed to allocate image memory");
        unsafe { device.bind_image_memory(image, allocation.memory(), allocation.offset()) }
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
            allocation: ManuallyDrop::new(allocation),
            mip_levels: create_info.mip_levels,
            format: create_info.format,
        }
    }
}