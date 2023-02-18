use std::sync::Arc;
use ash::vk;
use crate::etna;
use crate::etna::{CommandPool, image_transitions, SwapchainResult};
use crate::model::Model;

const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub struct FrameRenderer {
    device: Arc<etna::Device>,
    command_buffers: Vec<vk::CommandBuffer>,
    // sync objects
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,

    current_frame: usize,
}

impl FrameRenderer {
    pub fn draw_frame(&mut self, swapchain: &etna::Swapchain, pipeline: &etna::Pipeline, model: &Model) -> SwapchainResult<()> {
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

        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_stages)
            .signal_semaphores(signal_semaphores)
            .command_buffers(command_buffers);
        let submits = &[submit_info.build()];

        unsafe { self.device.queue_submit(self.device.graphics_queue, submits, self.current_in_flight_fence()) }
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

    fn record_draw_commands(&self, swapchain: &etna::Swapchain, pipeline: &etna::Pipeline, image_index: u32, model: &Model) {
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

        let color_attachments = &[color_attachment_info.build()];
        let rendering_info = vk::RenderingInfo::builder()
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: swapchain.extent(),
            })
            .layer_count(1)
            .color_attachments(color_attachments);

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
        unsafe { self.device.cmd_bind_vertex_buffers(command_buffer, 0, buffers, offsets) };
        unsafe { self.device.cmd_bind_index_buffer(command_buffer, model.index_buffer.buffer, 0, vk::IndexType::UINT16) };

        // TODO don't use hardcoded vertex count, instead use a model vert count
        unsafe { self.device.cmd_draw_indexed(command_buffer, 6, 1, 0, 0, 0) };
        unsafe { self.device.cmd_end_rendering(command_buffer) };

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
    pub fn create(device: Arc<etna::Device>, command_pool: &CommandPool) -> FrameRenderer {
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

        FrameRenderer {
            device,
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
        }
    }
}
