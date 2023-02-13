use std::ffi::CString;
use std::path::Path;
use std::sync::Arc;
use ash::vk;
use crate::{etna};
use crate::etna::shader::load_shader_module_from_file;

pub struct Pipeline {
    device: Arc<etna::Device>,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}

impl Pipeline {
    pub fn new(device: Arc<etna::Device>, swapchain: &etna::Swapchain) -> Pipeline {
        let vert_shader_module = load_shader_module_from_file(&device, Path::new("shaders/spirv/shader.vert_spv"));
        let frag_shader_module = load_shader_module_from_file(&device, Path::new("shaders/spirv/shader.frag_spv"));

        let main_function_name = CString::new("main").unwrap();
        let vertex_shader_stage_ci = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_shader_module)
            .name(main_function_name.as_c_str());
        let vertex_input_ci = vk::PipelineVertexInputStateCreateInfo::builder();


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
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0);

        let multisample_state_ci = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .min_sample_shading(1.0)
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

        let color_attachment_formats = &[swapchain.image_format];
        let mut pipeline_rendering_create_info = vk::PipelineRenderingCreateInfo::builder()
            .color_attachment_formats(color_attachment_formats);

        let pipeline_layout_ci = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&[])
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
            .subpass(0);
        let pipeline_create_infos = &[pipeline_ci.build()];
        let pipeline = unsafe { device.create_graphics_pipelines(vk::PipelineCache::null(), pipeline_create_infos, None) }
            .expect("Failed to create graphics pipeline")[0];

        unsafe { device.destroy_shader_module(vert_shader_module, None); }
        unsafe { device.destroy_shader_module(frag_shader_module, None); }

        Pipeline {
            device,
            pipeline_layout,
            pipeline,
        }
    }

    pub fn graphics_pipeline(&self) -> vk::Pipeline {
        self.pipeline
    }
}