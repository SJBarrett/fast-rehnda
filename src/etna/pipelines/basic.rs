use std::ffi::CString;
use std::mem::size_of;
use std::path::Path;
use ash::vk;
use crate::core::{ConstPtr, Mat4};
use crate::etna::{Device, GraphicsSettings, HostMappedBuffer, Swapchain};
use crate::etna::pipelines::{DescriptorAllocator, DescriptorBuilder, DescriptorLayoutCache, Pipeline, PipelineCreateInfo, PipelineMultisamplingInfo, PipelineVertexInputDescription};
use crate::etna::shader::{ ShaderModule};
use crate::scene::{Model, Vertex, ViewProjectionMatrices};

pub fn textured_pipeline(device: ConstPtr<Device>, descriptor_layout_cache: &mut DescriptorLayoutCache, descriptor_allocator: &mut DescriptorAllocator, graphics_settings: &GraphicsSettings, swapchain: &Swapchain, model: &Model, camera_buffer: &HostMappedBuffer) -> Pipeline {
    // TODO make this a global descriptor buffer
    let descriptor_buffer_info = vk::DescriptorBufferInfo::builder()
        .buffer(camera_buffer.vk_buffer())
        .offset(0)
        .range(size_of::<ViewProjectionMatrices>() as u64);
    let image_info = vk::DescriptorImageInfo::builder()
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .image_view(model.texture.image.image_view)
        .sampler(model.texture.sampler);
    let (descriptor_set, descriptor_set_layout) = DescriptorBuilder::begin(descriptor_layout_cache, descriptor_allocator)
        .bind_buffer(0, descriptor_buffer_info, vk::DescriptorType::UNIFORM_BUFFER, vk::ShaderStageFlags::VERTEX)
        .bind_image(1, image_info, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT)
        .build()
        .expect("Failed to allocate bindings");

    let vert_shader_module = ShaderModule::load_from_file(device, Path::new("shaders/spirv/shader.vert_spv"));
    let frag_shader_module = ShaderModule::load_from_file(device, Path::new("shaders/spirv/shader.frag_spv"));
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

    let vertex_attributes = Vertex::attribute_descriptions();
    let vertex_input = PipelineVertexInputDescription {
        bindings: &[Vertex::binding_description()],
        attributes: vertex_attributes.as_slice(),
    };
    let push_constant = vk::PushConstantRange::builder()
        .offset(0)
        .size(size_of::<Mat4>() as u32)
        .stage_flags(vk::ShaderStageFlags::VERTEX)
        .build();

    let multisampling = PipelineMultisamplingInfo {
        msaa_samples: graphics_settings.msaa_samples,
        enable_sample_rate_shading: graphics_settings.sample_rate_shading_enabled,
    };

    let create_info = PipelineCreateInfo {
        descriptor_set_layouts: &[descriptor_set_layout],
        descriptor_sets: &[descriptor_set],
        shader_stages: &[vertex_shader_stage_ci, frag_shader_stage_ci],
        push_constants: &[push_constant],
        extent: swapchain.extent,
        image_format: swapchain.image_format,
        vertex_input,
        multisampling,
    };

    Pipeline::create(device, &create_info)
}