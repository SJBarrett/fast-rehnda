use std::ffi::CString;
use std::mem::size_of;
use std::path::Path;

use ash::vk;

use crate::core::{ConstPtr, Mat4};
use crate::etna::{Device, GraphicsSettings, Swapchain};
use crate::etna::material_pipeline::{DescriptorManager, MaterialPipeline, PipelineCreateInfo, PipelineMultisamplingInfo, PipelineVertexInputDescription};
use crate::etna::shader::ShaderModule;
use crate::scene::{Model, Vertex};

pub fn textured_pipeline(device: ConstPtr<Device>, descriptor_manager: &mut DescriptorManager, graphics_settings: &GraphicsSettings, swapchain: &Swapchain, model: &Model) -> MaterialPipeline {
    let image_info = vk::DescriptorImageInfo::builder()
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .image_view(model.texture.image.image_view)
        .sampler(model.texture.sampler);
    let (descriptor_set, descriptor_set_layout) = descriptor_manager.descriptor_builder()
        .bind_image(0, image_info, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT)
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
        global_set_layout: &descriptor_manager.global_descriptor_layout,
        texture_set_layout: &descriptor_set_layout,
        texture_set: &descriptor_set,
        shader_stages: &[vertex_shader_stage_ci, frag_shader_stage_ci],
        push_constants: &[push_constant],
        extent: swapchain.extent,
        image_format: swapchain.image_format,
        vertex_input,
        multisampling,
    };

    MaterialPipeline::create(device, &create_info)
}