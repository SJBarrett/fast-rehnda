use std::path::Path;

use ash::vk;
use image::EncodableLayout;

use crate::core::ConstPtr;
use crate::etna;
use crate::etna::{Buffer, BufferCreateInfo, CommandPool, Device, Image, image_transitions, ImageCreateInfo, PhysicalDevice};

pub struct Texture {
    device: ConstPtr<Device>,
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
    pub fn create(device: ConstPtr<Device>, physical_device: &PhysicalDevice, command_pool: &CommandPool, image_path: &Path) -> Texture {
        let img = image::open(image_path).expect("Failed to open image");
        let rgba_img = img.to_rgba8();
        let src_buffer = Buffer::create_buffer_with_data(device, physical_device, BufferCreateInfo {
            data: rgba_img.as_bytes(),
            usage: vk::BufferUsageFlags::TRANSFER_SRC,
            memory_properties: vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        });
        let mip_levels = (rgba_img.width().max(rgba_img.height())).ilog2() + 1;
        let image = Image::create_image(device, physical_device, &ImageCreateInfo {
            width: rgba_img.width(),
            height: rgba_img.height(),
            mip_levels,
            format: vk::Format::R8G8B8A8_SRGB,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            memory_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
            image_aspect_flags: vk::ImageAspectFlags::COLOR,
            num_samples: vk::SampleCountFlags::TYPE_1,
        });


        let command_buffer = command_pool.one_time_command_buffer();
        image_transitions::transition_image_layout(&device, &command_buffer, image.vk_image, &image_transitions::TransitionProps::undefined_to_transfer_dst(mip_levels));

        // let command_buffer = command_pool.one_time_command_buffer();
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

        Self::generate_mipmaps(&device, physical_device, &image, rgba_img.width(), rgba_img.height(), mip_levels, *command_buffer);

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
            .min_lod(0.0)
            .max_lod(mip_levels as f32)
            .mip_lod_bias(0.0);

        let sampler = unsafe { device.create_sampler(&sampler_create_info, None) }
            .expect("Failed to create sampler for Texture");

        Texture {
            device,
            image,
            sampler,
        }
    }

    fn generate_mipmaps(device: &Device, physical_device: &PhysicalDevice, image: &etna::Image, width: u32, height: u32, mip_levels: u32, command_buffer: vk::CommandBuffer) {
        let format_properties = physical_device.get_format_properties(image.format);
        if (format_properties.optimal_tiling_features & vk::FormatFeatureFlags::SAMPLED_IMAGE_FILTER_LINEAR).is_empty() {
            panic!("Texture image format does not support linear blitting!");
        }
        let mut mip_width = width as i32;
        let mut mip_height = height as i32;
        for i in 1..mip_levels {
            // image was just copied into (transfer dst) and now we want to prepare to make it the source for blitting
            image_transitions::transition_image_layout(device, &command_buffer, image.vk_image, &image_transitions::TransitionProps {
                old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                new_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                src_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                dst_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                src_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                dst_access_mask: vk::AccessFlags2::TRANSFER_READ,
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: i - 1,
                level_count: 1,
            });

            let image_blit = vk::ImageBlit::builder()
                .src_offsets([
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D { x: mip_width, y: mip_height, z: 1 },
                ])
                .src_subresource(vk::ImageSubresourceLayers::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .mip_level(i - 1)
                    .base_array_layer(0)
                    .layer_count(1)
                    .build()
                )
                .dst_offsets([
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D { x: if mip_width > 1 { mip_width / 2 } else { 1 }, y: if mip_height > 1 { mip_height / 2 } else { 1 }, z: 1 },
                ])
                .dst_subresource(vk::ImageSubresourceLayers::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .mip_level(i)
                    .base_array_layer(0)
                    .layer_count(1)
                    .build()
                );

            unsafe {
                device.cmd_blit_image(
                    command_buffer,
                    image.vk_image,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    image.vk_image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    std::slice::from_ref(&image_blit), vk::Filter::LINEAR)
            };

            // now the image has been used to form the below mip level it can be prepared for being used in a shader
            image_transitions::transition_image_layout(device, &command_buffer, image.vk_image, &image_transitions::TransitionProps {
                old_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                src_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                dst_stage_mask: vk::PipelineStageFlags2::FRAGMENT_SHADER,
                src_access_mask: vk::AccessFlags2::TRANSFER_READ,
                dst_access_mask: vk::AccessFlags2::SHADER_READ,
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: i - 1,
                level_count: 1,
            });

            if mip_width > 1 {
                mip_width /= 2;
            }
            if mip_height > 1 {
                mip_height /= 2;
            }
        }

        image_transitions::transition_image_layout(device, &command_buffer, image.vk_image, &image_transitions::TransitionProps {
            old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            src_stage_mask: vk::PipelineStageFlags2::TRANSFER,
            dst_stage_mask: vk::PipelineStageFlags2::FRAGMENT_SHADER,
            src_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
            dst_access_mask: vk::AccessFlags2::SHADER_READ,
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: mip_levels - 1,
            level_count: 1,
        });
    }
}
