use std::ffi::CString;
use std::mem::size_of;
use std::path::Path;
use ash::vk;
use crate::core::{ConstPtr, Mat4};
use crate::etna::{Device, GraphicsSettings, Swapchain};
use crate::etna::pipelines::{Pipeline, PipelineCreateInfo, PipelineMultisamplingInfo, PipelineVertexInputDescription};
use crate::etna::shader::{ ShaderModule};
use crate::scene::Vertex;

pub fn basic_pipeline(device: ConstPtr<Device>, graphics_settings: &GraphicsSettings, swapchain: &Swapchain) -> Pipeline {
    let transformation_matrices_layout_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(0)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::VERTEX)
        .build();
    let sampler_layout_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(1)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT)
        .build();

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
        descriptor_sets: &[transformation_matrices_layout_binding, sampler_layout_binding],
        shader_stages: &[vertex_shader_stage_ci, frag_shader_stage_ci],
        push_constants: &[push_constant],
        extent: swapchain.extent,
        image_format: swapchain.image_format,
        vertex_input,
        multisampling,
    };

    Pipeline::create(device, &create_info)
}