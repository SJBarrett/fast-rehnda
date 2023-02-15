use std::sync::Arc;
use ash::vk;
use crate::etna;
use crate::etna::{Buffer, QueueFamilyIndices, SwapchainResult};

const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub struct FrameRenderer {
    device: Arc<etna::Device>,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    // sync objects
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,

    current_frame: usize,
}

impl FrameRenderer {
    pub fn draw_frame(&mut self, swapchain: &etna::Swapchain, pipeline: &etna::Pipeline, vertex_buffer: &Buffer) -> SwapchainResult<()> {
        let image_index = self.prepare_to_draw(swapchain)?;

        self.record_draw_commands(swapchain, pipeline, image_index, vertex_buffer);

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

    fn record_draw_commands(&self, swapchain: &etna::Swapchain, pipeline: &etna::Pipeline, image_index: u32, vertex_buffer: &Buffer) {
        let command_buffer = self.current_command_buffer();
        let begin_info = vk::CommandBufferBeginInfo::builder();
        unsafe { self.device.begin_command_buffer(command_buffer, &begin_info) }
            .expect("Failed to being recording command buffer");

        // with dynamic rendering we need to make the output image ready for writing to
        self.transition_image_layout(swapchain.images[image_index as usize], &TransitionProps {
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

        let buffers = &[vertex_buffer.buffer];
        let offsets = &[0u64];
        unsafe { self.device.cmd_bind_vertex_buffers(command_buffer, 0, buffers, offsets) };

        // TODO don't use hardcoded vertex count, instead use a model vert count
        unsafe { self.device.cmd_draw(command_buffer, 3, 1, 0, 0) };
        unsafe { self.device.cmd_end_rendering(command_buffer) };

        // For dynamic rendering we must manually transition the image layout for presentation
        // after drawing. This means changing it from a "color attachment write" to a "present".
        // This happens at the very last stage of render (i.e. BOTTOM_OF_PIPE)
        self.transition_image_layout(swapchain.images[image_index as usize], &TransitionProps {
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

    fn transition_image_layout(&self, image: vk::Image, transition: &TransitionProps) {
        let image_memory_barrier = vk::ImageMemoryBarrier2::builder()
            .src_access_mask(transition.src_access_mask)
            .src_stage_mask(transition.src_stage_mask)
            .old_layout(transition.old_layout)
            .new_layout(transition.new_layout)
            .dst_stage_mask(transition.dst_stage_mask)
            .dst_access_mask(transition.dst_access_mask)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1)
                .build()
            );
        let image_mem_barriers = &[image_memory_barrier.build()];
        let dep_info = vk::DependencyInfo::builder()
            .image_memory_barriers(image_mem_barriers);
        // make the transition to present happen
        unsafe { self.device.cmd_pipeline_barrier2(self.current_command_buffer(), &dep_info) };
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

struct TransitionProps {
    old_layout: vk::ImageLayout,
    src_access_mask: vk::AccessFlags2,
    src_stage_mask: vk::PipelineStageFlags2,
    new_layout: vk::ImageLayout,
    dst_access_mask: vk::AccessFlags2,
    dst_stage_mask: vk::PipelineStageFlags2,
}

// initialisation
impl FrameRenderer {
    pub fn create(device: Arc<etna::Device>, queue_family_indices: &QueueFamilyIndices) -> FrameRenderer {
        let command_pool_ci = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family_indices.graphics_family);
        let command_pool = unsafe { device.create_command_pool(&command_pool_ci, None) }
            .expect("Failed to create command pool");

        let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .command_buffer_count(MAX_FRAMES_IN_FLIGHT as u32)
            .level(vk::CommandBufferLevel::PRIMARY);
        let command_buffers = unsafe { device.allocate_command_buffers(&command_buffer_alloc_info) }
            .expect("Failed to allocation command buffer");

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
            command_pool,
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
            self.device.destroy_command_pool(self.command_pool, None);
        }
    }
}
