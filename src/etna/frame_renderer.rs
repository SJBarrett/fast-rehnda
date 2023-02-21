use std::mem::size_of;
use std::sync::Arc;
use std::time::Instant;
use ash::vk;
use lazy_static::lazy_static;
use crate::core::{Mat4, Vec3};
use crate::etna;
use crate::etna::{CommandPool, DepthBuffer, HostMappedBuffer, HostMappedBufferCreateInfo, image_transitions, Pipeline, SwapchainResult};
use crate::model::{Model, TransformationMatrices};

const MAX_FRAMES_IN_FLIGHT: usize = 2;
lazy_static! {
    static ref RENDERING_START_TIME: Instant = Instant::now();
}

pub struct FrameRenderer {
    device: Arc<etna::Device>,
    descriptor_pool: vk::DescriptorPool,
    depth_buffer: DepthBuffer,
    descriptor_sets: Vec<vk::DescriptorSet>,
    command_buffers: Vec<vk::CommandBuffer>,
    // sync objects
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
    uniform_buffers: Vec<HostMappedBuffer>,
    current_frame: usize,
}

impl FrameRenderer {
    pub fn resize(&mut self, physical_device: &etna::PhysicalDevice, command_pool: &CommandPool,  new_size: vk::Extent2D) {
        self.depth_buffer = DepthBuffer::create(self.device.clone(), physical_device, command_pool, new_size);
    }

    pub fn draw_frame(&mut self, swapchain: &etna::Swapchain, pipeline: &Pipeline, model: &Model) -> SwapchainResult<()> {
        let image_index = self.prepare_to_draw(swapchain)?;

        self.record_draw_commands(swapchain, pipeline, image_index, model);

        self.submit_draw(swapchain, image_index)?;
        Ok(())
    }

    fn submit_draw(&mut self, swapchain: &etna::Swapchain, image_index: u32) -> SwapchainResult<()> {
        // we need swapchain image to be available before we reach the color output stage (fragment shader)
        // so vertex shading could start before this point
        let wait_semaphores = &[self.current_image_available_semaphore()];
        let wait_stages = &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = &[self.current_render_finished_semaphore()];
        let command_buffers = &[self.current_command_buffer()];

        // update uniforms
        self.update_uniforms(swapchain.extent.width as f32 / swapchain.extent.height as f32);

        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_stages)
            .signal_semaphores(signal_semaphores)
            .command_buffers(command_buffers);

        unsafe { self.device.queue_submit(self.device.graphics_queue, std::slice::from_ref(&submit_info), self.current_in_flight_fence()) }
            .expect("Failed to submit to graphics queue");
        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
        swapchain.present(image_index, signal_semaphores)
    }

    fn prepare_to_draw(&self, swapchain: &etna::Swapchain) -> SwapchainResult<u32> {
        unsafe { self.device.wait_for_fences(&[self.current_in_flight_fence()], true, u64::MAX) }
            .expect("Failed to wait for in flight fence");

        unsafe { self.device.reset_command_buffer(self.current_command_buffer(), vk::CommandBufferResetFlags::empty()) }
            .expect("Failed to reset command buffer");

        let image_index = swapchain.acquire_next_image_and_get_index(self.current_image_available_semaphore())?;
        unsafe { self.device.reset_fences(&[self.current_in_flight_fence()]) }
            .expect("Failed to reset fences");
        Ok(image_index)
    }

    fn update_uniforms(&mut self, aspect_ratio: f32) {
        let seconds_elapsed = RENDERING_START_TIME.elapsed().as_secs_f32();
        let mut projection = Mat4::perspective_rh(45.0f32.to_radians(), aspect_ratio, 0.1, 10.0);
        // flip the y axis (assuming math is OpenGl style)
        projection.y_axis[1] *= -1.0;
        // OPTIMISATION Use push constants for transformation matrices
        let transformation_matrices = TransformationMatrices {
            model: Mat4::from_rotation_z(seconds_elapsed * 90.0f32.to_radians()),
            view: Mat4::look_at_rh(Vec3::new(2.0, 2.0, 2.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0)),
            projection,
        };
        let transformations = &[transformation_matrices];
        let buffer_data: &[u8] = bytemuck::cast_slice(transformations);
        self.uniform_buffers[self.current_frame].write_data(buffer_data);
    }

    fn record_draw_commands(&self, swapchain: &etna::Swapchain, pipeline: &Pipeline, image_index: u32, model: &Model) {
        let command_buffer = self.current_command_buffer();
        let begin_info = vk::CommandBufferBeginInfo::builder();
        unsafe { self.device.begin_command_buffer(command_buffer, &begin_info) }
            .expect("Failed to being recording command buffer");

        // with dynamic rendering we need to make the output image ready for writing to
        image_transitions::transition_image_layout(&self.device, &self.current_command_buffer(), swapchain.images[image_index as usize], &image_transitions::TransitionProps {
            old_layout: vk::ImageLayout::UNDEFINED,
            src_access_mask: vk::AccessFlags2::empty(),
            src_stage_mask: vk::PipelineStageFlags2::TOP_OF_PIPE,
            new_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            dst_access_mask: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
            dst_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
        });

        let clear_color = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0]
            }
        };

        let color_attachment_info = vk::RenderingAttachmentInfo::builder()
            .image_view(swapchain.image_views()[image_index as usize])
            .image_layout(vk::ImageLayout::ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(clear_color);
        let depth_attachment_info = vk::RenderingAttachmentInfo::builder()
            .image_view(self.depth_buffer.image.image_view)
            .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .clear_value(vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 }
            });

        let rendering_info = vk::RenderingInfo::builder()
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: swapchain.extent(),
            })
            .layer_count(1)
            .color_attachments(std::slice::from_ref(&color_attachment_info))
            .depth_attachment(&depth_attachment_info);

        unsafe { self.device.cmd_begin_rendering(command_buffer, &rendering_info); }
        unsafe { self.device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline.graphics_pipeline()); }

        let viewport = [vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(swapchain.extent().width as f32)
            .height(swapchain.extent().height as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build()];
        unsafe { self.device.cmd_set_viewport(command_buffer, 0, &viewport); }

        let scissor = [vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(swapchain.extent())
            .build()];
        unsafe { self.device.cmd_set_scissor(command_buffer, 0, &scissor); }

        let buffers = &[model.vertex_buffer.buffer];
        let offsets = &[0u64];

        unsafe {
            self.device.cmd_bind_vertex_buffers(command_buffer, 0, buffers, offsets);
            self.device.cmd_bind_index_buffer(command_buffer, model.index_buffer.buffer, 0, vk::IndexType::UINT16);

            let descriptor_sets = &[self.descriptor_sets[self.current_frame]];
            self.device.cmd_bind_descriptor_sets(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline_layout, 0, descriptor_sets, &[]);
            // TODO don't use hardcoded vertex count, instead use a model vert count
            self.device.cmd_draw_indexed(command_buffer, model.index_count, 1, 0, 0, 0);
            self.device.cmd_end_rendering(command_buffer);
        }


        // For dynamic rendering we must manually transition the image layout for presentation
        // after drawing. This means changing it from a "color attachment write" to a "present".
        // This happens at the very last stage of render (i.e. BOTTOM_OF_PIPE)
        image_transitions::transition_image_layout(&self.device, &self.current_command_buffer(), swapchain.images[image_index as usize], &image_transitions::TransitionProps {
            old_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            src_access_mask: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
            src_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            new_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            dst_stage_mask: vk::PipelineStageFlags2::BOTTOM_OF_PIPE,
            dst_access_mask: vk::AccessFlags2::empty(),
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
        });

        unsafe { self.device.end_command_buffer(command_buffer) }
            .expect("Failed to record command buffer");
    }
    
    fn current_command_buffer(&self) -> vk::CommandBuffer {
        self.command_buffers[self.current_frame]
    }

    fn current_image_available_semaphore(&self) -> vk::Semaphore {
        self.image_available_semaphores[self.current_frame]
    }

    fn current_render_finished_semaphore(&self) -> vk::Semaphore {
        self.render_finished_semaphores[self.current_frame]
    }
    
    fn current_in_flight_fence(&self) -> vk::Fence {
        self.in_flight_fences[self.current_frame]
    }
}

// initialisation
impl FrameRenderer {
    pub fn create(device: Arc<etna::Device>, physical_device: &etna::PhysicalDevice, pipeline: &Pipeline, command_pool: &CommandPool, extent: vk::Extent2D, model: &Model) -> FrameRenderer {
        let command_buffers = command_pool.allocate_command_buffers(MAX_FRAMES_IN_FLIGHT as u32);

        let semaphore_ci = vk::SemaphoreCreateInfo::builder().build();
        let signaled_fence_ci = vk::FenceCreateInfo::builder()
            .flags(vk::FenceCreateFlags::SIGNALED);

        let mut image_available_semaphores: Vec<vk::Semaphore> = Vec::new();
        let mut render_finished_semaphores: Vec<vk::Semaphore> = Vec::new();
        let mut in_flight_fences: Vec<vk::Fence> = Vec::new();

        for _i in 0..MAX_FRAMES_IN_FLIGHT {
            image_available_semaphores.push(unsafe { device.create_semaphore(&semaphore_ci, None) }
                .expect("Failed to create semaphore"));
            render_finished_semaphores.push(unsafe { device.create_semaphore(&semaphore_ci, None) }
                .expect("Failed to create semaphore"));
            in_flight_fences.push(unsafe { device.create_fence(&signaled_fence_ci, None) }
                .expect("Failed to create fence"));
        }

        let uniform_buffers: Vec<HostMappedBuffer> = (0..MAX_FRAMES_IN_FLIGHT).map(|_| {
            HostMappedBuffer::create(device.clone(), physical_device, HostMappedBufferCreateInfo {
                size: size_of::<TransformationMatrices>() as u64,
                usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
            })
        }).collect();

        let transform_ub_pool_size = vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(MAX_FRAMES_IN_FLIGHT as u32)
            .build();
        let texture_sampler_pool_size = vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(MAX_FRAMES_IN_FLIGHT as u32)
            .build();
        let pool_sizes = &[transform_ub_pool_size, texture_sampler_pool_size];
        let descriptor_pool_ci = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(pool_sizes)
            .max_sets(MAX_FRAMES_IN_FLIGHT as u32);
        let descriptor_pool = unsafe { device.create_descriptor_pool(&descriptor_pool_ci, None) }
            .expect("Failed to create descriptor pool");

        let set_layouts: Vec<vk::DescriptorSetLayout> = (0..MAX_FRAMES_IN_FLIGHT).map(|_| pipeline.descriptor_set_layout).collect();
        let descriptor_set_alloc_infos = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_pool)
                .set_layouts(set_layouts.as_slice());
        let descriptor_sets = unsafe { device.allocate_descriptor_sets(&descriptor_set_alloc_infos) }
            .expect("Failed to allocate descriptor sets");

        for i in 0..MAX_FRAMES_IN_FLIGHT {
            let descriptor_buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(uniform_buffers[i].vk_buffer())
                .offset(0)
                .range(size_of::<TransformationMatrices>() as u64);
            let image_info = vk::DescriptorImageInfo::builder()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(model.texture.image.image_view)
                .sampler(model.texture.sampler);
            let write_transforms_set = vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_sets[i])
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(std::slice::from_ref(&descriptor_buffer_info))
                .build();
            let write_image_set = vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_sets[i])
                .dst_binding(1)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&image_info))
                .build();
            let write_sets = &[write_transforms_set, write_image_set];
            unsafe { device.update_descriptor_sets(write_sets, &[]); }
        }

        let depth_buffer = DepthBuffer::create(device.clone(), physical_device, command_pool, extent);
        FrameRenderer {
            device,
            uniform_buffers,
            depth_buffer,
            descriptor_pool,
            descriptor_sets,
            command_buffers,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            current_frame: 0,
        }
    }
}

impl Drop for FrameRenderer {
    fn drop(&mut self) {
        unsafe {
            self.image_available_semaphores.iter().for_each(|semaphore| self.device.destroy_semaphore(*semaphore, None));
            self.render_finished_semaphores.iter().for_each(|semaphore| self.device.destroy_semaphore(*semaphore, None));
            self.in_flight_fences.iter().for_each(|fence| self.device.destroy_fence(*fence, None));
            self.device.destroy_descriptor_pool(self.descriptor_pool, None);
        }
    }
}
