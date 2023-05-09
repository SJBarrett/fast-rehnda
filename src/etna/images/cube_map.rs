use std::ffi::CString;
use std::mem::size_of;
use std::ops::Deref;
use std::path::Path;
use ash::vk;
use ash::vk::{DescriptorSet, Extent2D};
use bytemuck_derive::{Pod, Zeroable};
use image::{EncodableLayout};
use lazy_static::lazy_static;
use crate::assets::{cube, vulkan_projection_matrix};
use crate::etna::{Buffer, BufferCreateInfo, CommandPool, Device, FramebufferCreateInfo, GraphicsSettings, Image, image_transitions, ImageCreateInfo, ImageType, MsaaSamples, PhysicalDevice, SamplerOptions, TexSamplerOptions, Texture, TextureCreateInfo};
use crate::etna::image_transitions::{transition_image_layout, TransitionProps};
use crate::etna::material_pipeline::{DescriptorManager, layout_binding, MaterialPipeline, PipelineCreateInfo, PipelineMultisamplingInfo, PipelineVertexInputDescription, RasterizationOptions};
use crate::etna::shader::ShaderModule;
use crate::rehnda_core::{ConstPtr, Mat4};

pub struct CubeMapTexture {
    device: ConstPtr<Device>,
    pub image: Image,
    pub sampler: vk::Sampler,
    pub descriptor_set: vk::DescriptorSet,
}

impl Drop for CubeMapTexture {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_sampler(self.sampler, None);
        }
    }
}

impl CubeMapTexture {
    pub fn create(device: ConstPtr<Device>, image: Image, descriptor_manager: &mut DescriptorManager) -> Self {
        let sampler_ci = vk::SamplerCreateInfo::builder()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            // only use anisotropy if the feature is enabled
            .anisotropy_enable(false)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .min_lod(0.0)
            .max_lod(1.0)
            .mip_lod_bias(0.0)
            .build()
            ;
        let sampler = unsafe { device.create_sampler(&sampler_ci, None) }.unwrap();


        let cube_map_image_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(image.image_view)
            .sampler(sampler);


        let (descriptor_set, _descriptor_set_layout) = descriptor_manager.descriptor_builder()
            .bind_image(0, cube_map_image_info, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT)
            .build()
            .expect("Failed to allocate bindings");
        Self {
            device,
            image,
            sampler,
            descriptor_set,
        }
    }
}

pub struct CubeMapManager {
    device: ConstPtr<Device>,
    pub cube_map_pipeline: MaterialPipeline,
    pub cube_vertex_buffer: Buffer,
}

impl CubeMapManager {
    pub fn create(device: ConstPtr<Device>, descriptor_manager: &mut DescriptorManager, command_pool: &CommandPool) -> Self {
        Self {
            device,
            cube_map_pipeline: cube_map_pipeline(device, descriptor_manager, &GraphicsSettings {
                msaa_samples: MsaaSamples::X1,
                sample_rate_shading_enabled: false,
            }),
            cube_vertex_buffer: Buffer::create_and_initialize_buffer_with_staging_buffer(device, command_pool, BufferCreateInfo {
                data: cube::CUBE_VERTICES.as_slice().as_bytes(),
                usage: vk::BufferUsageFlags::VERTEX_BUFFER,
            })
        }
    }

    pub fn create_cube_image(&self, physical_device: &PhysicalDevice, command_pool: &CommandPool, descriptor_manager: &mut DescriptorManager, path: &Path) -> Image {
        let cube_image = Image::create_image(self.device, &ImageCreateInfo {
            image_type: ImageType::Cube,
            width: 4096,
            height: 4096,
            format: vk::Format::R32G32B32A32_SFLOAT,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            mip_levels: 1,
            memory_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
            image_aspect_flags: vk::ImageAspectFlags::COLOR,
            num_samples: vk::SampleCountFlags::TYPE_1,
            create_flags: vk::ImageCreateFlags::CUBE_COMPATIBLE,
        });
        let mut individual_views = Vec::with_capacity(6);
        for i in 0..6usize {
            let view_ci = vk::ImageViewCreateInfo::builder()
                .image(cube_image.vk_image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .subresource_range(vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(i as u32)
                    .layer_count(1)
                    .build()
                )
                ;
           individual_views.push(unsafe { self.device.create_image_view(&view_ci, None) }.unwrap());
        }
        let transition_buffer = command_pool.one_time_command_buffer();

        transition_image_layout(&self.device, &transition_buffer, cube_image.vk_image,&TransitionProps {
            old_layout: vk::ImageLayout::UNDEFINED,
            new_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            src_stage_mask: vk::PipelineStageFlags2::TOP_OF_PIPE,
            dst_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: vk::AccessFlags2::empty(),
            dst_access_mask: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            layer_count: 6,
        });

        let img = image::open(path).unwrap();
        let data = img.to_rgba32f();
        let equirectangular_texture = Texture::create(self.device, physical_device, command_pool, descriptor_manager, &TextureCreateInfo {
            width: img.width(),
            height: img.height(),
            format: vk::Format::R32G32B32A32_SFLOAT,
            mip_levels: None,
            data: data.as_bytes(),
            sampler_info: SamplerOptions::FilterOptions(&TexSamplerOptions {
                min_filter: Some(vk::Filter::LINEAR),
                mag_filter: Some(vk::Filter::LINEAR),
                mip_map_mode: None,
                address_mode_u: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                address_mode_v: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            }),
        });


        let equirectangular_image_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(equirectangular_texture.image.image_view)
            .sampler(equirectangular_texture.sampler);
        let (descriptor_set, _set_layout) = descriptor_manager.descriptor_builder()
            .bind_image(0, equirectangular_image_info, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT)
            .build()
            .expect("Failed to build binding");

        let projection_matrix = vulkan_projection_matrix(90.0f32.to_radians(), 1.0, 0.1, 10.0);

        for i in 0..6 {
            draw_cube_face(&self.device, &self.cube_map_pipeline, descriptor_set, command_pool, &DrawCubeFaceInfo {
                cube_face_view: individual_views[i],
                cube_vertex_buffer: &self.cube_vertex_buffer,
                resolution: 4096,
                projection_matrix,
                view_matrix: CUBE_CAPTURE_VIEWS[i],
            });
        }

        // transition texture for sampling later
        image_transitions::transition_image_layout(&self.device, &*transition_buffer, cube_image.vk_image, &TransitionProps {
            old_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            src_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            dst_stage_mask: vk::PipelineStageFlags2::FRAGMENT_SHADER,
            src_access_mask: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
            dst_access_mask: vk::AccessFlags2::SHADER_SAMPLED_READ,
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            layer_count: 6,
        });

        for view in individual_views {
            unsafe { self.device.destroy_image_view(view, None); }
        }

        cube_image
    }
}

struct DrawCubeFaceInfo<'a> {
    cube_face_view: vk::ImageView,
    cube_vertex_buffer: &'a Buffer,
    resolution: u32,
    projection_matrix: Mat4,
    view_matrix: Mat4,
}

fn draw_cube_face(device: &Device, pipeline: &MaterialPipeline, equirectangular_texture_descriptor: DescriptorSet, command_pool: &CommandPool, draw_info: &DrawCubeFaceInfo) {
    let one_time_command_buffer = command_pool.one_time_command_buffer();
    let command_buffer = *one_time_command_buffer;

    // ------------------ setup the render pass ------------------
    let clear_color = vk::ClearValue {
        color: vk::ClearColorValue {
            float32: [0.52, 0.8, 0.92, 1.0]
        }
    };
    let color_attachment_info = vk::RenderingAttachmentInfo::builder()
        .image_view(draw_info.cube_face_view)
        .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .resolve_mode(vk::ResolveModeFlags::NONE)
        .clear_value(clear_color);
    let rendering_info = vk::RenderingInfo::builder()
        .render_area(vk::Rect2D {
            offset: vk::Offset2D { x:0, y: 0 },
            extent: vk::Extent2D { width: draw_info.resolution, height: draw_info.resolution },
        })
        .layer_count(1)
        .color_attachments(std::slice::from_ref(&color_attachment_info));
    unsafe { device.cmd_begin_rendering(command_buffer, &rendering_info) };
    // ----------------------------------------------------------

    // bind the cube map pipeline
    unsafe { device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline.graphics_pipeline())}
    let viewport = [vk::Viewport::builder()
        .x(0.0)
        .y(0.0)
        .width(draw_info.resolution as f32)
        .height(draw_info.resolution as f32)
        .min_depth(0.0)
        .max_depth(1.0)
        .build()];
    unsafe { device.cmd_set_viewport(command_buffer, 0, &viewport); }

    let scissor = [vk::Rect2D::builder()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(Extent2D { width: draw_info.resolution, height: draw_info.resolution })
        .build()];
    unsafe { device.cmd_set_scissor(command_buffer, 0, &scissor); }

    // bind the cube vertex data (we are drawing this without indices
    unsafe {
        device.cmd_bind_vertex_buffers(command_buffer, 0, std::slice::from_ref(&draw_info.cube_vertex_buffer.buffer), std::slice::from_ref(&0u64));
        device.cmd_bind_descriptor_sets(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline_layout, 0, &[equirectangular_texture_descriptor], &[]);
    }

    // draw
    let push_constant = CubeMapShaderPushConstant {
        projection_matrix: draw_info.projection_matrix,
        view_matrix: draw_info.view_matrix,
    };
    let push_data: &[u8] = bytemuck::cast_slice(std::slice::from_ref(&push_constant));
    unsafe {
        device.cmd_push_constants(command_buffer, pipeline.pipeline_layout, vk::ShaderStageFlags::VERTEX, 0, push_data);
        device.cmd_draw(command_buffer, cube::CUBE_VERTICES.len() as u32, 1, 0, 0);
    }

    // ------------------  end the render pass ------------------
    unsafe { device.cmd_end_rendering(command_buffer) };
}

pub struct CubeMap {
    cube_image: Image,
    sampler: vk::Sampler,
}

fn cube_map_pipeline(device: ConstPtr<Device>, descriptor_manager: &mut DescriptorManager, graphics_settings: &GraphicsSettings) -> MaterialPipeline {
    let equirectangular_map_sampler = descriptor_manager.layout_cache.create_descriptor_layout_for_binding(&[
        layout_binding(0, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT),
    ]);
    let vert_shader_module = ShaderModule::load_from_file(device, Path::new("shaders/spirv/cubemap.vert_spv"));
    let frag_shader_module = ShaderModule::load_from_file(device, Path::new("shaders/spirv/cubemap.frag_spv"));
    let main_function_name = CString::new("main").unwrap();
    let vertex_shader_stage_ci = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::VERTEX)
        .module(vert_shader_module.handle())
        .name(main_function_name.as_c_str())
        .build();
    let frag_shader_stage_ci = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::FRAGMENT)
        .module(frag_shader_module.handle())
        .name(main_function_name.as_c_str())
        .build();

    let model_matrix_push_constant = vk::PushConstantRange::builder()
        .offset(0)
        .size(size_of::<CubeMapShaderPushConstant>() as u32)
        .stage_flags(vk::ShaderStageFlags::VERTEX)
        .build();

    let multisampling = PipelineMultisamplingInfo {
        msaa_samples: graphics_settings.msaa_samples,
        enable_sample_rate_shading: graphics_settings.sample_rate_shading_enabled,
    };

    let vertex_attributes = cube::cube_vertex_attributes();
    let vertex_input = PipelineVertexInputDescription {
        bindings: &[cube::cube_vertex_input_bindings()],
        attributes: vertex_attributes.as_slice(),
    };

    let create_info = PipelineCreateInfo {
        global_set_layouts: &[],
        additional_descriptor_set_layouts: &[equirectangular_map_sampler],
        shader_stages: &[vertex_shader_stage_ci, frag_shader_stage_ci],
        push_constants: &[model_matrix_push_constant],
        extent: Extent2D { width: 128, height: 128 },
        image_format: vk::Format::R32G32B32A32_SFLOAT,
        vertex_input,
        multisampling,
        rasterization_options: &RasterizationOptions::default(),
    };

    MaterialPipeline::create(device, &create_info)
}

#[repr(C)]
#[derive(Zeroable, Pod, Debug, Copy, Clone)]
pub struct CubeMapShaderPushConstant {
    pub projection_matrix: Mat4,
    pub view_matrix: Mat4,
}

lazy_static! {
    static ref CUBE_CAPTURE_VIEWS: [Mat4; 6] = [
        Mat4::look_at_rh((0.0, 0.0, 0.0).into(), (1.0, 0.0, 0.0).into(), (0.0, -1.0, 0.0).into()),
        Mat4::look_at_rh((0.0, 0.0, 0.0).into(), (-1.0, 0.0, 0.0).into(), (0.0, -1.0, 0.0).into()),
        Mat4::look_at_rh((0.0, 0.0, 0.0).into(), (0.0, -1.0, 0.0).into(), (0.0, 0.0, -1.0).into()),
        Mat4::look_at_rh((0.0, 0.0, 0.0).into(), (0.0, 1.0, 0.0).into(), (0.0, 0.0, 1.0).into()),
        Mat4::look_at_rh((0.0, 0.0, 0.0).into(), (0.0, 0.0, 1.0).into(), (0.0, -1.0, 0.0).into()),
        Mat4::look_at_rh((0.0, 0.0, 0.0).into(), (0.0, 0.0, -1.0).into(), (0.0, -1.0, 0.0).into()),
    ];
}
