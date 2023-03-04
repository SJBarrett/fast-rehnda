use ash::vk;

use crate::rehnda_core::ConstPtr;
use crate::etna;
use crate::etna::MsaaSamples;

pub struct MaterialPipeline {
    device: ConstPtr<etna::Device>,
    pub pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
}

impl Drop for MaterialPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
            // layouts are destroyed by the layout cache
        }
    }
}

pub struct PipelineCreateInfo<'a> {
    pub global_set_layouts: &'a [vk::DescriptorSetLayout],
    pub additional_descriptor_set_layouts: &'a [vk::DescriptorSetLayout],
    pub shader_stages: &'a [vk::PipelineShaderStageCreateInfo],
    pub vertex_input: PipelineVertexInputDescription<'a>,
    pub push_constants: &'a [vk::PushConstantRange],
    pub image_format: vk::Format,
    pub extent: vk::Extent2D,
    pub multisampling: PipelineMultisamplingInfo,
    pub rasterization_options: &'a RasterizationOptions,
}

pub struct RasterizationOptions {
    pub cull_mode: vk::CullModeFlags,
}

impl Default for RasterizationOptions {
    fn default() -> Self {
        RasterizationOptions {
            cull_mode: vk::CullModeFlags::BACK
        }
    }
}

pub struct PipelineMultisamplingInfo {
    pub msaa_samples: MsaaSamples,
    pub enable_sample_rate_shading: bool,
}

pub struct PipelineVertexInputDescription<'a> {
    pub bindings: &'a [vk::VertexInputBindingDescription],
    pub attributes: &'a [vk::VertexInputAttributeDescription],
}

impl MaterialPipeline {
    pub fn create(device: ConstPtr<etna::Device>, create_info: &PipelineCreateInfo) -> MaterialPipeline {
        let vertex_input_ci = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(create_info.vertex_input.bindings)
            .vertex_attribute_descriptions(create_info.vertex_input.attributes);

        // let us change viewport and scissor state without rebuilding the pipeline
        let input_assembly_ci = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

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
            .line_width(1.0)
            .cull_mode(create_info.rasterization_options.cull_mode)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0);

        let multisample_state_ci = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(create_info.multisampling.msaa_samples.to_sample_count_flags())
            .sample_shading_enable(create_info.multisampling.enable_sample_rate_shading)
            .min_sample_shading(if create_info.multisampling.enable_sample_rate_shading { 0.2 } else { 1.0 }) // closer to 1 is smoother
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

        MaterialPipeline {
            device,
            pipeline_layout,
            pipeline,
        }
    }

    pub fn graphics_pipeline(&self) -> vk::Pipeline {
        self.pipeline
    }
}