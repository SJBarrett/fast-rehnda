use std::sync::Arc;
use ash::vk;
use crate::etna;
use crate::etna::QueueFamilyIndices;

pub struct FrameRenderer {
    device: Arc<etna::Device>,
    swapchain_framebuffers: Vec<vk::Framebuffer>,
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    // sync objects
    image_available_semaphore: vk::Semaphore,
    render_finished_semaphore: vk::Semaphore,
    in_flight_fence: vk::Fence,
}

impl FrameRenderer {
    pub fn draw_frame(&self, swapchain: &etna::Swapchain, pipeline: &etna::Pipeline) {
        let image_index = self.prepare_to_draw(swapchain);

        self.record_draw_commands(swapchain, pipeline, image_index);

        self.submit_draw(swapchain, image_index);
    }

    fn submit_draw(&self, swapchain: &etna::Swapchain, image_index: u32) {
        // we need swapchain image to be available before we reach the color output stage (fragment shader)
        // so vertex shading could start before this point
        let wait_semaphores = &[self.image_available_semaphore];
        let wait_stages = &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = &[self.render_finished_semaphore];
        let command_buffers = &[self.command_buffer];

        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_stages)
            .signal_semaphores(signal_semaphores)
            .command_buffers(command_buffers);
        let submits = &[submit_info.build()];

        unsafe { self.device.queue_submit(self.device.graphics_queue, submits, self.in_flight_fence) }
            .expect("Failed to submit to graphics queue");
        swapchain.present(image_index, signal_semaphores);
    }

    fn prepare_to_draw(&self, swapchain: &etna::Swapchain) -> u32 {
        unsafe { self.device.wait_for_fences(&[self.in_flight_fence], true, u64::MAX) }
            .expect("Failed to wait for in flight fence");
        unsafe { self.device.reset_fences(&[self.in_flight_fence]) }
            .expect("Failed to reset fences");
        unsafe { self.device.reset_command_buffer(self.command_buffer, vk::CommandBufferResetFlags::empty()) }
            .expect("Failed to reset command buffer");

        swapchain.acquire_next_image_and_get_index(self.image_available_semaphore)
    }

    fn record_draw_commands(&self, swapchain: &etna::Swapchain, pipeline: &etna::Pipeline, image_index: u32) {
        let begin_info = vk::CommandBufferBeginInfo::builder();
        unsafe { self.device.begin_command_buffer(self.command_buffer, &begin_info) }
            .expect("Failed to being recording command buffer");

        let clear_color = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0]
            }
        };
        let clear_values = &[clear_color];
        let render_pass_info = vk::RenderPassBeginInfo::builder()
            .render_pass(swapchain.render_pass())
            .framebuffer(self.swapchain_framebuffers[image_index as usize])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: swapchain.extent(),
            })
            .clear_values(clear_values)
            ;

        unsafe { self.device.cmd_begin_render_pass(self.command_buffer, &render_pass_info, vk::SubpassContents::INLINE); }
        unsafe { self.device.cmd_bind_pipeline(self.command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline.graphics_pipeline()); }

        let viewport = [vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(swapchain.extent().width as f32)
            .height(swapchain.extent().height as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build()];
        unsafe { self.device.cmd_set_viewport(self.command_buffer, 0, &viewport); }

        let scissor = [vk::Rect2D::builder()
            .offset(vk::Offset2D {x: 0, y: 0})
            .extent(swapchain.extent())
            .build()];
        unsafe { self.device.cmd_set_scissor(self.command_buffer, 0, &scissor); }

        unsafe { self.device.cmd_draw(self.command_buffer, 3, 1, 0, 0) };
        unsafe { self.device.cmd_end_render_pass(self.command_buffer) };
        unsafe { self.device.end_command_buffer(self.command_buffer) }
            .expect("Failed to record command buffer");
    }
}

// initialisation
impl FrameRenderer {
    pub fn create(device: Arc<etna::Device>, swapchain: &etna::Swapchain, queue_family_indices: &QueueFamilyIndices) -> FrameRenderer {
        let swapchain_framebuffers: Vec<vk::Framebuffer> = swapchain.image_views().iter().map(|image_view| {
            let attachments = [*image_view];

            let framebuffer_ci = vk::FramebufferCreateInfo::builder()
                .render_pass(swapchain.render_pass())
                .attachments(&attachments)
                .width(swapchain.extent().width)
                .height(swapchain.extent().height)
                .layers(1);
            unsafe { device.create_framebuffer(&framebuffer_ci, None) }
                .expect("Failed to create framebuffer")
        }).collect();

        let command_pool_ci = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family_indices.graphics_family);
        let command_pool = unsafe { device.create_command_pool(&command_pool_ci, None) }
            .expect("Failed to create command pool");

        let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);
        let command_buffer = unsafe { device.allocate_command_buffers(&command_buffer_alloc_info) }
            .expect("Failed to allocation command buffer")[0];

        let semaphore_ci = vk::SemaphoreCreateInfo::builder().build();
        let signaled_fence_ci = vk::FenceCreateInfo::builder()
            .flags(vk::FenceCreateFlags::SIGNALED);

        let image_available_semaphore = unsafe { device.create_semaphore(&semaphore_ci, None) }
            .expect("Failed to create semaphore");
        let render_finished_semaphore = unsafe { device.create_semaphore(&semaphore_ci, None) }
            .expect("Failed to create semaphore");
        let in_flight_fence = unsafe { device.create_fence(&signaled_fence_ci, None) }
            .expect("Failed to create fence");

        FrameRenderer {
            device,
            swapchain_framebuffers,
            command_pool,
            command_buffer,
            image_available_semaphore,
            render_finished_semaphore,
            in_flight_fence
        }
    }
}

impl Drop for FrameRenderer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_semaphore(self.image_available_semaphore, None);
            self.device.destroy_semaphore(self.render_finished_semaphore, None);
            self.device.destroy_fence(self.in_flight_fence, None);
            self.device.destroy_command_pool(self.command_pool, None);
            for framebuffer in &self.swapchain_framebuffers {
                self.device.destroy_framebuffer(*framebuffer, None);
            }
        }
    }
}
