use std::path::Path;
use std::sync::Arc;
use ash::vk;
use image::EncodableLayout;
use crate::etna::{Buffer, BufferCreateInfo, CommandPool, Device, image_transitions, PhysicalDevice};

pub struct Texture {
    device: Arc<Device>,
    texture_image: vk::Image,
    texture_memory: vk::DeviceMemory,
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image(self.texture_image, None);
            self.device.free_memory(self.texture_memory, None);
        }
    }
}

impl Texture {
    pub fn create(device: Arc<Device>, physical_device: &PhysicalDevice, command_pool: &CommandPool, image_path: &Path) -> Texture {
        let img = image::open(image_path).expect("Failed to open image");
        let rgba_img = img.to_rgba8();
        let src_buffer = Buffer::create_buffer_with_data(device.clone(), physical_device, BufferCreateInfo {
            size: rgba_img.as_bytes().len() as u64,
            usage: vk::BufferUsageFlags::TRANSFER_SRC,
            memory_properties: vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        }, rgba_img.as_bytes());

        let (texture_image, texture_memory) = Self::create_image(&device, physical_device, &ImageCreateInfo {
            width: rgba_img.width(),
            height: rgba_img.height(),
            format: vk::Format::R8G8B8A8_SINT,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            memory_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
        });

        let command_buffer = command_pool.one_time_command_buffer();

        image_transitions::transition_image_layout(&device, &command_buffer, texture_image, &image_transitions::TransitionProps::undefined_to_transfer_dst());

        let copy_region = vk::BufferImageCopy::builder()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(vk::ImageSubresourceLayers::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .mip_level(0)
                .base_array_layer(0)
                .layer_count(1).build()
            )
            .image_offset(vk::Offset3D {
                x: 0,
                y: 0,
                z: 0,
            })
            .image_extent(vk::Extent3D {
                width: rgba_img.width(),
                height: rgba_img.height(),
                depth: 1,
            })
            .build();
        let copy_regions = &[copy_region];

        unsafe { device.cmd_copy_buffer_to_image(*command_buffer, src_buffer.buffer, texture_image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, copy_regions) };
        image_transitions::transition_image_layout(&device, &command_buffer, texture_image, &image_transitions::TransitionProps::transfer_dst_to_shader_read());

        Texture {
            device,
            texture_image,
            texture_memory,
        }
    }

    fn create_image(device: &Device, physical_device: &PhysicalDevice, create_info: &ImageCreateInfo) -> (vk::Image, vk::DeviceMemory) {
        let image_ci = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                width: create_info.width,
                height: create_info.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .format(create_info.format)
            .tiling(create_info.tiling)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(create_info.usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1)
            ;

        let texture_image = unsafe { device.create_image(&image_ci, None) }
            .expect("Failed to create image for texture");

        let memory_requirements = unsafe { device.get_image_memory_requirements(texture_image) };
        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(physical_device.find_memory_type(memory_requirements.memory_type_bits, create_info.memory_properties));
        let texture_memory = unsafe { device.allocate_memory(&alloc_info, None) }
            .expect("Failed to allocate memory for texture");
        unsafe { device.bind_image_memory(texture_image, texture_memory, 0) }
            .expect("Failed to bind image memory for texture");

        (texture_image, texture_memory)
    }
}

struct ImageCreateInfo {
    width: u32,
    height: u32,
    format: vk::Format,
    tiling: vk::ImageTiling,
    usage: vk::ImageUsageFlags,
    memory_properties: vk::MemoryPropertyFlags,
}