use std::ffi::CString;
use std::mem::size_of;
use std::path::Path;

use ash::vk;
use egui::epaint::Vertex;
use memoffset::offset_of;

use crate::etna::{Device, GraphicsSettings, Swapchain};
use crate::etna::material_pipeline::{DescriptorManager, layout_binding, PipelineCreateInfo, PipelineMultisamplingInfo, PipelineVertexInputDescription, RasterizationOptions};
use crate::etna::shader::ShaderModule;
use crate::rehnda_core::ConstPtr;

pub fn egui_pipeline(device: ConstPtr<Device>, descriptor_manager: &mut DescriptorManager, graphics_settings: &GraphicsSettings, swapchain: &Swapchain) -> UiPipeline {
    let texture_binding_description = descriptor_manager.layout_cache.create_descriptor_layout_for_binding(&layout_binding(0, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT));
    let vert_shader_module = ShaderModule::load_from_file(device, Path::new("shaders/spirv/egui.vert_spv"));
    let frag_shader_module = ShaderModule::load_from_file(device, Path::new("shaders/spirv/egui.frag_spv"));
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

    let vertex_attributes = egui_vertex_descriptions();
    let vertex_input = PipelineVertexInputDescription {
        bindings: &[egui_binding_description()],
        attributes: vertex_attributes.as_slice(),
    };
    // push constant for pushing screen size
    let push_constant = vk::PushConstantRange::builder()
        .offset(0)
        .size((size_of::<u32>() * 2) as u32)
        .stage_flags(vk::ShaderStageFlags::VERTEX)
        .build();

    let multisampling = PipelineMultisamplingInfo {
        msaa_samples: graphics_settings.msaa_samples,
        enable_sample_rate_shading: graphics_settings.sample_rate_shading_enabled,
    };

    let create_info = PipelineCreateInfo {
        global_set_layouts: &[],
        additional_descriptor_set_layouts: &[texture_binding_description],
        shader_stages: &[vertex_shader_stage_ci, frag_shader_stage_ci],
        push_constants: &[push_constant],
        extent: swapchain.extent,
        image_format: swapchain.image_format,
        vertex_input,
        multisampling,
        rasterization_options: &RasterizationOptions {
            cull_mode: vk::CullModeFlags::NONE,
        },
    };

    create_ui_pipeline(device, &create_info)
}

fn egui_vertex_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
    vec![
        // position attribute
        vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(offset_of!(Vertex, pos) as u32)
            .build(),
        // uv attribute
        vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(offset_of!(Vertex, uv) as u32)
            .build(),
        // color attribute
        vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::R8G8B8A8_UNORM)
            .offset(offset_of!(Vertex, color) as u32)
            .build(),
    ]
}

fn egui_binding_description() -> vk::VertexInputBindingDescription {
    vk::VertexInputBindingDescription::builder()
        .binding(0)
        .stride(size_of::<Vertex>() as u32)
        .input_rate(vk::VertexInputRate::VERTEX)
        .build()
}



pub struct UiPipeline {
    device: ConstPtr<Device>,
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
}

impl Drop for UiPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
            // layouts are destroyed by the layout cache
        }
    }
}

pub fn create_ui_pipeline(device: ConstPtr<Device>, create_info: &PipelineCreateInfo) -> UiPipeline {
    let vertex_input_ci = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_binding_descriptions(create_info.vertex_input.bindings)
        .vertex_attribute_descriptions(create_info.vertex_input.attributes);

    // let us change viewport and scissor state without rebuilding the pipeline
    let input_assembly_ci = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

    let viewport = vk::Viewport::builder()
        .x(0.0)
        .y(0.0)
        .width(create_info.extent.width as f32)
        .height(create_info.extent.height as f32)
        .min_depth(0.0)
        .max_depth(1.0);
    let viewports = &[viewport.build()];

    let scissor = vk::Rect2D::builder()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(create_info.extent);
    let scissors = &[scissor.build()];

    let viewport_state_ci = vk::PipelineViewportStateCreateInfo::builder()
        .viewports(viewports)
        .scissors(scissors);

    let dynamic_state_ci = vk::PipelineDynamicStateCreateInfo::builder()
        .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

    let rasterization_ci = vk::PipelineRasterizationStateCreateInfo::builder()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false)
        .line_width(1.0);

    let multisample_state_ci = vk::PipelineMultisampleStateCreateInfo::builder()
        .rasterization_samples(create_info.multisampling.msaa_samples.to_sample_count_flags())
        .sample_shading_enable(create_info.multisampling.enable_sample_rate_shading)
        .min_sample_shading(if create_info.multisampling.enable_sample_rate_shading { 0.2 } else { 1.0 }) // closer to 1 is smoother
        .alpha_to_coverage_enable(false)
        .alpha_to_one_enable(false);

    let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
        .color_write_mask(vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A)
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::ONE)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA);
    let color_blend_attachments = &[color_blend_attachment.build()];

    let color_blend_state_ci = vk::PipelineColorBlendStateCreateInfo::builder()
        .attachments(color_blend_attachments);
    // stencil op
    let stencil_op = vk::StencilOpState::builder()
        .fail_op(vk::StencilOp::KEEP)
        .pass_op(vk::StencilOp::KEEP)
        .compare_op(vk::CompareOp::ALWAYS)
        .build();
    let depth_stencil_ci = vk::PipelineDepthStencilStateCreateInfo::builder()
        .depth_test_enable(true)
        .depth_write_enable(true)
        .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
        .depth_bounds_test_enable(false)
        .stencil_test_enable(false)
        .front(stencil_op)
        .back(stencil_op);

    let color_attachment_formats = &[create_info.image_format];
    let mut pipeline_rendering_create_info = vk::PipelineRenderingCreateInfo::builder()
        .color_attachment_formats(color_attachment_formats)
        .depth_attachment_format(vk::Format::D32_SFLOAT); // TODO don't assume this format

    let set_layouts: Vec<vk::DescriptorSetLayout> = [create_info.global_set_layouts, create_info.additional_descriptor_set_layouts].concat();
    let pipeline_layout_ci = vk::PipelineLayoutCreateInfo::builder()
        .set_layouts(set_layouts.as_slice())
        .push_constant_ranges(create_info.push_constants);

    let pipeline_layout = unsafe { device.create_pipeline_layout(&pipeline_layout_ci, None) }
        .expect("Failed to create pipline layout");

    let pipeline_ci = vk::GraphicsPipelineCreateInfo::builder()
        .stages(create_info.shader_stages)
        .vertex_input_state(&vertex_input_ci)
        .input_assembly_state(&input_assembly_ci)
        .viewport_state(&viewport_state_ci)
        .rasterization_state(&rasterization_ci)
        .multisample_state(&multisample_state_ci)
        .color_blend_state(&color_blend_state_ci)
        .dynamic_state(&dynamic_state_ci)
        .layout(pipeline_layout)
        .render_pass(vk::RenderPass::null())
        .push_next(&mut pipeline_rendering_create_info)
        .depth_stencil_state(&depth_stencil_ci)
        .subpass(0);
    let pipeline_create_infos = &[pipeline_ci.build()];
    let pipeline = unsafe { device.create_graphics_pipelines(vk::PipelineCache::null(), pipeline_create_infos, None) }
        .expect("Failed to create graphics pipeline")[0];

    UiPipeline {
        device,
        pipeline_layout,
        pipeline,
    }
}
