use std::ffi::CString;
use std::path::Path;

use ash::vk;

use crate::etna;
use crate::core::ConstPtr;
use crate::etna::shader::load_shader_module_from_file;
use crate::model::Vertex;

pub struct Pipeline {
    device: ConstPtr<etna::Device>,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}

impl Pipeline {
    pub fn new(device: ConstPtr<etna::Device>, graphics_settings: &etna::GraphicsSettings, swapchain: &etna::Swapchain) -> Pipeline {
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
        let bindings = &[transformation_matrices_layout_binding, sampler_layout_binding];
        let descriptor_set_layout_ci = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(bindings);
        let descriptor_set_layout = unsafe { device.create_descriptor_set_layout(&descriptor_set_layout_ci, None) }
            .expect("Failed to create descriptor set layout");

        let vert_shader_module = load_shader_module_from_file(&device, Path::new("shaders/spirv/shader.vert_spv"));
        let frag_shader_module = load_shader_module_from_file(&device, Path::new("shaders/spirv/shader.frag_spv"));

        let main_function_name = CString::new("main").unwrap();
        let vertex_shader_stage_ci = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_shader_module)
            .name(main_function_name.as_c_str());
        let vertex_binding_descriptions = [Vertex::binding_description()];
        let vertex_attribute_descriptions = Vertex::attribute_descriptions();
        let vertex_input_ci = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&vertex_binding_descriptions)
            .vertex_attribute_descriptions(vertex_attribute_descriptions.as_slice())
            ;


        let frag_shader_stage_ci = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_shader_module)
            .name(main_function_name.as_c_str());
        let shader_stages = &[vertex_shader_stage_ci.build(), frag_shader_stage_ci.build()];

        // let us change viewport and scissor state without rebuilding the pipeline

        let input_assembly_ci = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(swapchain.extent().width as f32)
            .height(swapchain.extent().height as f32)
            .min_depth(0.0)
            .max_depth(1.0);
        let viewports = &[viewport.build()];

        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(swapchain.extent());
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
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0);

        let multisample_state_ci = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(graphics_settings.msaa_samples.to_sample_count_flags())
            .sample_shading_enable(graphics_settings.sample_rate_shading_enabled)
            .min_sample_shading(if graphics_settings.sample_rate_shading_enabled { 0.2 } else { 1.0 }) // closer to 1 is smoother
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false);

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A)
            .blend_enable(false)
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(vk::BlendFactor::ZERO)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD);
        let color_blend_attachments = &[color_blend_attachment.build()];

        let color_blend_state_ci = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(color_blend_attachments)
            .blend_constants([0.0, 0.0, 0.0, 0.0]);

        let depth_stencil_ci = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);

        let color_attachment_formats = &[swapchain.image_format];
        let mut pipeline_rendering_create_info = vk::PipelineRenderingCreateInfo::builder()
            .color_attachment_formats(color_attachment_formats)
            .depth_attachment_format(vk::Format::D32_SFLOAT); // TODO don't assume this format

        let set_layouts = &[descriptor_set_layout];
        let pipeline_layout_ci = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(set_layouts)
            .push_constant_ranges(&[]);

        let pipeline_layout = unsafe { device.create_pipeline_layout(&pipeline_layout_ci, None) }
            .expect("Failed to create pipline layout");

        let pipeline_ci = vk::GraphicsPipelineCreateInfo::builder()
            .stages(shader_stages)
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

        unsafe { device.destroy_shader_module(vert_shader_module, None); }
        unsafe { device.destroy_shader_module(frag_shader_module, None); }

        Pipeline {
            device,
            pipeline_layout,
            descriptor_set_layout,
            pipeline,
        }
    }

    pub fn graphics_pipeline(&self) -> vk::Pipeline {
        self.pipeline
    }
}