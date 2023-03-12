use std::ffi::CString;
use std::mem::size_of;
use std::path::Path;

use ash::vk;

use crate::rehnda_core::{ConstPtr, Mat4};
use crate::etna::{Device, GraphicsSettings, Swapchain};
use crate::etna::material_pipeline::{DescriptorManager, layout_binding, MaterialPipeline, PipelineCreateInfo, PipelineMultisamplingInfo, PipelineVertexInputDescription, RasterizationOptions};
use crate::etna::shader::ShaderModule;
use crate::scene::{Vertex};

pub fn textured_pipeline(device: ConstPtr<Device>, descriptor_manager: &mut DescriptorManager, graphics_settings: &GraphicsSettings, swapchain: &Swapchain) -> MaterialPipeline {
    let base_color_texture_sampler_layout = descriptor_manager.layout_cache.create_descriptor_layout_for_binding(&[
        layout_binding(0, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT),
        layout_binding(1, vk::DescriptorType::UNIFORM_BUFFER, vk::ShaderStageFlags::FRAGMENT),
    ]);
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
        global_set_layouts: &[descriptor_manager.global_descriptor_layout],
        additional_descriptor_set_layouts: &[base_color_texture_sampler_layout],
        shader_stages: &[vertex_shader_stage_ci, frag_shader_stage_ci],
        push_constants: &[push_constant],
        extent: swapchain.extent,
        image_format: swapchain.image_format,
        vertex_input,
        multisampling,
        rasterization_options: &RasterizationOptions::default(),
    };

    MaterialPipeline::create(device, &create_info)
}

pub fn non_textured_pipeline(device: ConstPtr<Device>, descriptor_manager: &mut DescriptorManager, graphics_settings: &GraphicsSettings, swapchain: &Swapchain) -> MaterialPipeline {
    let vert_shader_module = ShaderModule::load_from_file(device, Path::new("shaders/spirv/shader.vert_spv"));
    let frag_shader_module = ShaderModule::load_from_file(device, Path::new("shaders/spirv/non_textured.frag_spv"));
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
        global_set_layouts: &[descriptor_manager.global_descriptor_layout],
        additional_descriptor_set_layouts: &[],
        shader_stages: &[vertex_shader_stage_ci, frag_shader_stage_ci],
        push_constants: &[push_constant],
        extent: swapchain.extent,
        image_format: swapchain.image_format,
        vertex_input,
        multisampling,
        rasterization_options: &RasterizationOptions::default(),
    };

    MaterialPipeline::create(device, &create_info)
}