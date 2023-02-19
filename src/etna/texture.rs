use std::path::Path;
use std::sync::Arc;
use ash::vk;
use image::EncodableLayout;
use crate::etna::{Buffer, BufferCreateInfo, CommandPool, Device, image_transitions, PhysicalDevice};

pub struct Image {
    device: Arc<Device>,
    pub vk_image: vk::Image,
    pub device_memory: vk::DeviceMemory,
    pub image_view: vk::ImageView,
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

impl Image {
    fn create_image(device: Arc<Device>, physical_device: &PhysicalDevice, create_info: &ImageCreateInfo) -> Image {
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
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
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
        }
    }
}

pub struct Texture {
    device: Arc<Device>,
    pub image: Image,
    pub sampler: vk::Sampler,
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_sampler(self.sampler, None);

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

        let image = Image::create_image(device.clone(), physical_device, &ImageCreateInfo {
            width: rgba_img.width(),
            height: rgba_img.height(),
            format: vk::Format::R8G8B8A8_SRGB,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            memory_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
        });

        let command_buffer = command_pool.one_time_command_buffer();

        image_transitions::transition_image_layout(&device, &command_buffer, image.vk_image, &image_transitions::TransitionProps::undefined_to_transfer_dst());

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

        unsafe { device.cmd_copy_buffer_to_image(*command_buffer, src_buffer.buffer, image.vk_image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, copy_regions) };
        image_transitions::transition_image_layout(&device, &command_buffer, image.vk_image, &image_transitions::TransitionProps::transfer_dst_to_shader_read());

        let sampler_create_info = vk::SamplerCreateInfo::builder()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            // only use anisotropy if the feature is enabled
            .anisotropy_enable(device.enabled_features.sampler_anisotropy == vk::TRUE)
            .max_anisotropy(if device.enabled_features.sampler_anisotropy == vk::TRUE {
                physical_device.device_properties.limits.max_sampler_anisotropy
            } else {
                1.0
            })
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .mip_lod_bias(0.0)
            .min_lod(0.0)
            .max_lod(0.0);

        let sampler = unsafe { device.create_sampler(&sampler_create_info, None) }
            .expect("Failed to create sampler for Texture");

        Texture {
            device,
            image,
            sampler,
        }
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