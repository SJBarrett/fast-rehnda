use std::ffi::CString;
use std::path::Path;
use ash::vk;
use ash::vk::DescriptorSet;
use bevy_ecs::component::Component;
use bevy_ecs::system::Resource;
use crate::assets::cube;
use crate::assets::material_server::MaterialPipelineHandle;
use crate::etna::{Device, GraphicsSettings, Swapchain};
use crate::etna::material_pipeline::{DescriptorManager, layout_binding, MaterialPipeline, PipelineCreateInfo, PipelineMultisamplingInfo, PipelineVertexInputDescription, RasterizationOptions};
use crate::etna::shader::ShaderModule;
use crate::rehnda_core::ConstPtr;

#[derive(Component)]
pub struct SkyBox {
    pub pipeline: MaterialPipelineHandle,
    pub descriptor_set: DescriptorSet,
}

pub fn skybox_pipeline(device: ConstPtr<Device>, descriptor_manager: &mut DescriptorManager, graphics_settings: &GraphicsSettings, swapchain: &Swapchain, vert_shader_path: &Path, frag_shader_path: &Path) -> MaterialPipeline {
    let sky_box_cube_sampler_set = descriptor_manager.layout_cache.create_descriptor_layout_for_binding(&[
        layout_binding(0, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT),
    ]);
    let vert_shader_module = ShaderModule::load_from_file(device, Path::new(vert_shader_path));
    let frag_shader_module = ShaderModule::load_from_file(device, Path::new(frag_shader_path));
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

    let vertex_attributes = cube::cube_vertex_attributes();
    let vertex_input = PipelineVertexInputDescription {
        bindings: &[cube::cube_vertex_input_bindings()],
        attributes: vertex_attributes.as_slice(),
    };

    let multisampling = PipelineMultisamplingInfo {
        msaa_samples: graphics_settings.msaa_samples,
        enable_sample_rate_shading: graphics_settings.sample_rate_shading_enabled,
    };

    let create_info = PipelineCreateInfo {
        global_set_layouts: &[descriptor_manager.global_descriptor_layout],
        additional_descriptor_set_layouts: &[sky_box_cube_sampler_set],
        shader_stages: &[vertex_shader_stage_ci, frag_shader_stage_ci],
        push_constants: &[],
        extent: swapchain.extent,
        image_format: swapchain.image_format,
        vertex_input,
        multisampling,
        rasterization_options: &RasterizationOptions::default(),
    };

    MaterialPipeline::create(device, &create_info)
}