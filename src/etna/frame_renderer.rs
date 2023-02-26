use std::mem::size_of;

use ash::vk;

use crate::core::ConstPtr;
use crate::etna;
use crate::etna::{CommandPool, Device, GraphicsSettings, HostMappedBuffer, HostMappedBufferCreateInfo, image_transitions, PhysicalDevice, Swapchain, SwapchainResult, vkinit};
use crate::etna::pipelines::Pipeline;
use crate::scene::{Camera, Model, Scene, ViewProjectionMatrices};

const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub struct FrameRenderer {
    device: ConstPtr<Device>,
    graphics_settings: GraphicsSettings,
    descriptor_pool: vk::DescriptorPool,
    frame_data: [FrameData; MAX_FRAMES_IN_FLIGHT],
    uniform_buffers: Vec<HostMappedBuffer>,
    current_frame: usize,
}

#[derive(Debug)]
struct FrameData {
    image_available_semaphore: vk::Semaphore,
    render_finished_semaphore: vk::Semaphore,
    in_flight_fence: vk::Fence,

    descriptor_set: vk::DescriptorSet,
    command_buffer: vk::CommandBuffer,
}

impl FrameRenderer {
    pub fn draw_frame(&mut self, swapchain: &Swapchain, pipeline: &Pipeline, scene: &Scene) -> SwapchainResult<()> {
        // update uniforms
        self.update_view_projection_ubo(&scene.camera);

        let frame_data = unsafe { self.frame_data.get_unchecked(self.current_frame % MAX_FRAMES_IN_FLIGHT) };

        // acquire the image from the swapcahin to draw to, waiting for the previous usage of this frame data to be free
        let image_index = prepare_to_draw(&self.device, swapchain, frame_data)?;

        unsafe { self.device.begin_command_buffer(frame_data.command_buffer, &vkinit::COMMAND_BUFFER_BEGIN_INFO) }
            .expect("Failed to being recording command buffer");

        cmd_begin_rendering(&self.device, swapchain, frame_data.command_buffer, image_index);
        draw_pipeline_and_models(&self.device, swapchain, pipeline, scene, frame_data);
        cmd_end_rendering(&self.device, swapchain, frame_data.command_buffer, image_index);

        unsafe { self.device.end_command_buffer(frame_data.command_buffer) }
            .expect("Failed to record command buffer");

        submit_draw(&self.device, swapchain, image_index, frame_data)?;

        self.current_frame += 1;
        Ok(())
    }

    fn update_view_projection_ubo(&mut self, camera: &Camera) {
        let view_proj = camera.to_view_proj();
        let buffer_data: &[u8] = bytemuck::cast_slice(std::slice::from_ref(&view_proj));
        self.uniform_buffers[self.current_frame % MAX_FRAMES_IN_FLIGHT].write_data(buffer_data);
    }
}

fn submit_draw(device: &Device, swapchain: &Swapchain, image_index: u32, frame_data: &FrameData) -> SwapchainResult<()> {
    // we need swapchain image to be available before we reach the color output stage (fragment shader)
    // so vertex shading could start before this point
    let signal_semaphores = &[frame_data.render_finished_semaphore];
    let submit_info = vk::SubmitInfo::builder()
        .wait_semaphores(std::slice::from_ref(&frame_data.image_available_semaphore))
        .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
        .signal_semaphores(signal_semaphores)
        .command_buffers(std::slice::from_ref(&frame_data.command_buffer));

    unsafe { device.queue_submit(device.graphics_queue, std::slice::from_ref(&submit_info), frame_data.in_flight_fence) }
        .expect("Failed to submit to graphics queue");
    swapchain.present(image_index, signal_semaphores)
}


fn prepare_to_draw(device: &Device, swapchain: &Swapchain, frame_data: &FrameData) -> SwapchainResult<u32> {
    unsafe { device.wait_for_fences(&[frame_data.in_flight_fence], true, u64::MAX) }
        .expect("Failed to wait for in flight fence");

    unsafe { device.reset_command_buffer(frame_data.command_buffer, vk::CommandBufferResetFlags::empty()) }
        .expect("Failed to reset command buffer");

    let image_index = swapchain.acquire_next_image_and_get_index(frame_data.image_available_semaphore)?;
    unsafe { device.reset_fences(&[frame_data.in_flight_fence]) }
        .expect("Failed to reset fences");

    Ok(image_index)
}

fn draw_pipeline_and_models(device: &Device, swapchain: &Swapchain, pipeline: &Pipeline, scene: &Scene, frame_data: &FrameData) {
    unsafe { device.cmd_bind_pipeline(frame_data.command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline.graphics_pipeline()); }
    let viewport = [vk::Viewport::builder()
        .x(0.0)
        .y(0.0)
        .width(swapchain.extent().width as f32)
        .height(swapchain.extent().height as f32)
        .min_depth(0.0)
        .max_depth(1.0)
        .build()];
    unsafe { device.cmd_set_viewport(frame_data.command_buffer, 0, &viewport); }

    let scissor = [vk::Rect2D::builder()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(swapchain.extent())
        .build()];
    unsafe { device.cmd_set_scissor(frame_data.command_buffer, 0, &scissor); }
    draw_model(device, frame_data, pipeline.pipeline_layout, &scene.model);
}

fn draw_model(device: &Device, frame_data: &FrameData, pipeline_layout: vk::PipelineLayout, model: &Model) {
    let buffers = &[model.vertex_buffer.buffer];
    let offsets = &[0u64];
    let command_buffer = frame_data.command_buffer;
    let model_data: &[u8] = bytemuck::cast_slice(std::slice::from_ref(&model.transform));
    unsafe {
        device.cmd_push_constants(command_buffer, pipeline_layout, vk::ShaderStageFlags::VERTEX, 0, model_data);
        device.cmd_bind_vertex_buffers(command_buffer, 0, buffers, offsets);
        device.cmd_bind_index_buffer(command_buffer, model.index_buffer.buffer, 0, vk::IndexType::UINT16);

        let descriptor_sets = &[frame_data.descriptor_set];
        device.cmd_bind_descriptor_sets(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline_layout, 0, descriptor_sets, &[]);
        // TODO don't use hardcoded vertex count, instead use a scene vert count
        device.cmd_draw_indexed(command_buffer, model.index_count, 1, 0, 0, 0);
    }
}

fn cmd_begin_rendering(device: &Device, swapchain: &Swapchain, command_buffer: vk::CommandBuffer, swapchain_image_index: u32) {
    // with dynamic rendering we need to make the output image ready for writing to
    image_transitions::transition_image_layout(device, &command_buffer, swapchain.images[swapchain_image_index as usize], &image_transitions::TransitionProps {
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
    let color_attachment_info = if swapchain.msaa_enabled {
        vk::RenderingAttachmentInfo::builder()
            .image_view(swapchain.color_image.image_view)
            .image_layout(vk::ImageLayout::ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .resolve_mode(vk::ResolveModeFlags::AVERAGE)
            .resolve_image_layout(vk::ImageLayout::ATTACHMENT_OPTIMAL)
            .resolve_image_view(swapchain.image_views[swapchain_image_index as usize])
            .clear_value(clear_color)
    } else {
        vk::RenderingAttachmentInfo::builder()
            .image_view(swapchain.image_views[swapchain_image_index as usize])
            .image_layout(vk::ImageLayout::ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .resolve_mode(vk::ResolveModeFlags::NONE)
            .clear_value(clear_color)
    };
    let depth_attachment = vk::RenderingAttachmentInfo::builder()
        .image_view(swapchain.depth_buffer.image.image_view)
        .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::DONT_CARE)
        .clear_value(vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 }
        });
    let rendering_info = vk::RenderingInfo::builder()
        .render_area(vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: swapchain.extent,
        })
        .layer_count(1)
        .color_attachments(std::slice::from_ref(&color_attachment_info))
        .depth_attachment(&depth_attachment);
    unsafe { device.cmd_begin_rendering(command_buffer, &rendering_info); }
}

fn cmd_end_rendering(device: &Device, swapchain: &Swapchain, command_buffer: vk::CommandBuffer, swapchain_image_index: u32) {
    unsafe { device.cmd_end_rendering(command_buffer); }

    // For dynamic rendering we must manually transition the image layout for presentation
    // after drawing. This means changing it from a "color attachment write" to a "present".
    // This happens at the very last stage of render (i.e. BOTTOM_OF_PIPE)
    image_transitions::transition_image_layout(device, &command_buffer, swapchain.images[swapchain_image_index as usize], &image_transitions::TransitionProps {
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
}

// initialisation
impl FrameRenderer {
    pub fn create(device: ConstPtr<etna::Device>, physical_device: &PhysicalDevice, pipeline: &Pipeline, command_pool: &CommandPool, swapchain: &Swapchain, model: &Model) -> FrameRenderer {
        let command_buffers = command_pool.allocate_command_buffers(MAX_FRAMES_IN_FLIGHT as u32);

        // TODO remove uniform buffers from here
        let uniform_buffers: Vec<HostMappedBuffer> = (0..MAX_FRAMES_IN_FLIGHT).map(|_| {
            HostMappedBuffer::create(device, physical_device, HostMappedBufferCreateInfo {
                size: size_of::<ViewProjectionMatrices>() as u64,
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

        let frame_data: [FrameData; 2] = (0..MAX_FRAMES_IN_FLIGHT).map(|i| {
            let image_available_semaphore = unsafe { device.create_semaphore(&vkinit::SEMAPHORE_CREATE_INFO, None) }
                .expect("Failed to create semaphore");
            let render_finished_semaphore = unsafe { device.create_semaphore(&vkinit::SEMAPHORE_CREATE_INFO, None) }
                .expect("Failed to create semaphore");
            let in_flight_fence = unsafe { device.create_fence(&vkinit::SIGNALED_FENCE_CREATE_INFO, None) }
                .expect("Failed to create fence");

            FrameData {
                image_available_semaphore,
                render_finished_semaphore,
                in_flight_fence,
                descriptor_set: descriptor_sets[i],
                command_buffer: command_buffers[i],
            }
        })
            .collect::<Vec<FrameData>>()
            .try_into()
            .unwrap();

        for i in 0..MAX_FRAMES_IN_FLIGHT {
            let descriptor_buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(uniform_buffers[i].vk_buffer())
                .offset(0)
                .range(size_of::<ViewProjectionMatrices>() as u64);
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
        FrameRenderer {
            device,
            graphics_settings: physical_device.graphics_settings,
            uniform_buffers,
            descriptor_pool,
            frame_data,
            current_frame: 0,
        }
    }
}

impl Drop for FrameRenderer {
    fn drop(&mut self) {
        unsafe {
            for frame_data in &self.frame_data {
                self.device.destroy_semaphore(frame_data.render_finished_semaphore, None);
                self.device.destroy_semaphore(frame_data.image_available_semaphore, None);
                self.device.destroy_fence(frame_data.in_flight_fence, None);
            }
            self.device.destroy_descriptor_pool(self.descriptor_pool, None);
        }
    }
}
