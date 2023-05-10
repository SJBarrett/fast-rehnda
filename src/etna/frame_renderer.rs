use std::fmt::{Debug, Formatter};
use std::mem::size_of;

use ash::vk;
use bevy_ecs::prelude::*;
use bevy_hierarchy::Children;
use gltf::json::Asset;

use crate::etna::{CommandPool, Device, HostMappedBuffer, HostMappedBufferCreateInfo, image_transitions, PhysicalDeviceRes, Swapchain, SwapchainResult, vkinit};
use crate::etna::material_pipeline::{DescriptorManager, MaterialPipeline, ModelPushConstants};
use crate::rehnda_core::{ConstPtr, Mat4};
use crate::assets::{AssetManager, Camera, cube, MeshHandle, ViewProjectionMatrices};
use crate::assets::demo_scenes::Actor;
use crate::assets::light_source::LightingDataManager;
use crate::assets::material_server::{MaterialPipelineHandle, MaterialServer};
use crate::assets::render_object::{MaterialHandle, Mesh, PbrMaterial, RenderObject, Transform};
use crate::etna::cube_map::EnvironmentMaps;
use crate::ui::{EguiOutput, UiPainter};

const MAX_FRAMES_IN_FLIGHT: usize = 2;

#[derive(Resource)]
pub struct FrameRenderContext {
    device: ConstPtr<Device>,
    frame_data: [FrameData; MAX_FRAMES_IN_FLIGHT],
    current_frame: usize,
}

struct FrameData {
    image_available_semaphore: vk::Semaphore,
    render_finished_semaphore: vk::Semaphore,
    in_flight_fence: vk::Fence,
    command_buffer: vk::CommandBuffer,

    global_data: HostMappedBuffer,
    global_descriptor: vk::DescriptorSet,
}

impl Debug for FrameData {
    fn fmt(&self, _: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

pub fn draw_system(
    mut frame_renderer: ResMut<FrameRenderContext>,
    physical_device: PhysicalDeviceRes,
    command_pool: Res<CommandPool>,
    mut swapchain: ResMut<Swapchain>,
    asset_manager: Res<AssetManager>,
    material_server: Res<MaterialServer>,
    camera: Res<Camera>,
    actors_query: Query<(&Transform, &Children), With<Actor>>,
    render_objects_query: Query<(&Transform, &RenderObject)>,
    mut ui_painter: ResMut<UiPainter>,
    ui_output: Res<EguiOutput>,
    lights: Res<LightingDataManager>,
) {
    let frame_data = unsafe { frame_renderer.frame_data.get_unchecked(frame_renderer.current_frame % MAX_FRAMES_IN_FLIGHT) };

    update_global_buffer(frame_data, &camera);

    // acquire the image from the swapcahin to draw to, waiting for the previous usage of this frame data to be free
    let image_index = match prepare_to_draw(&frame_renderer.device, &swapchain, frame_data) {
        Ok(index) => index,
        Err(_) => {
            swapchain.needs_recreation = true;
            return;
        }
    };

    unsafe { frame_renderer.device.begin_command_buffer(frame_data.command_buffer, &vkinit::COMMAND_BUFFER_BEGIN_INFO) }
        .expect("Failed to being recording command buffer");

    cmd_begin_rendering(&frame_renderer.device, &swapchain, frame_data.command_buffer, image_index);
    draw_sky_box(&frame_renderer.device, &swapchain, frame_data, &asset_manager, &material_server);
    let mut last_material_pipeline_handle = MaterialPipelineHandle::null();
    let mut last_material_pipeline: Option<&MaterialPipeline> = None;
    let mut last_material_handle = MaterialHandle::null();
    let mut last_mesh_handle = MeshHandle::null();
    let mut last_mesh: Option<&Mesh> = None;

    for (parent_transform, children) in actors_query.iter() {
        for child_render_object in children {
            if let Ok((render_object_relative_transform, render_object)) = render_objects_query.get(*child_render_object) {
                // TODO support relative transforms
                let mesh_handle = render_object.mesh_handle;
                let is_different_material = last_material_pipeline_handle.is_null() || last_material_pipeline_handle != render_object.material_pipeline_handle;
                if let Some(loaded_material) = material_server.material_ref(&render_object.material_pipeline_handle) {
                    if is_different_material {
                        last_material_pipeline = Some(loaded_material);
                        bind_material_pipeline(&frame_renderer.device, &swapchain, loaded_material, frame_data);
                    }
                } else {
                    continue;
                }

                let current_material = unsafe { last_material_pipeline.unwrap_unchecked() };
                // new model so bind model specific resources
                if last_mesh_handle.is_null() || last_mesh_handle != mesh_handle {
                    let mesh = asset_manager.mesh_ref(&mesh_handle);
                    last_mesh = Some(mesh);
                    bind_model(&frame_renderer.device, frame_data, mesh);
                }
                let mesh_material_handle = render_object.material_instance_handle;
                // new material so bind material specific resources
                if last_material_handle.is_null() || last_material_handle != mesh_material_handle {
                    let material = asset_manager.material_ref(&mesh_material_handle);
                    last_material_handle = mesh_material_handle;
                    bind_material(&frame_renderer.device, frame_data, current_material, material, &lights, &asset_manager.global_light_map.as_ref().unwrap().0);
                }

                let current_model = unsafe { last_mesh.unwrap_unchecked() };
                draw_object(&frame_renderer.device, frame_data, current_material, current_model, parent_transform.matrix());
                last_material_pipeline_handle = render_object.material_pipeline_handle;
                last_mesh_handle = mesh_handle;

            };
        }
    }

    ui_painter.update_resources(&physical_device, &command_pool, &ui_output);
    ui_painter.draw(&frame_renderer.device, &swapchain, frame_data.command_buffer, &ui_output);

    cmd_end_rendering(&frame_renderer.device, &swapchain, frame_data.command_buffer, image_index);

    unsafe { frame_renderer.device.end_command_buffer(frame_data.command_buffer) }
        .expect("Failed to record command buffer");

    if let Err(_) = submit_draw(&frame_renderer.device, &swapchain, image_index, frame_data) {
        swapchain.needs_recreation = true;
        return;
    };

    frame_renderer.current_frame += 1;
}

fn draw_sky_box(device: &Device, swapchain: &Swapchain, frame_data: &FrameData, asset_manager: &AssetManager, material_server: &MaterialServer) {
    if let Some((environment_maps, pipeline_handle)) = &asset_manager.global_light_map {
        let pipeline = &material_server.material_ref(pipeline_handle).unwrap();

        bind_material_pipeline(device, swapchain, pipeline, frame_data);
        unsafe {
            device.cmd_bind_descriptor_sets(frame_data.command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline_layout, 0, &[frame_data.global_descriptor, environment_maps.sky_box_texture.descriptor_set], &[]);
            device.cmd_bind_vertex_buffers(frame_data.command_buffer, 0, std::slice::from_ref(&asset_manager.cube_map_manager.cube_vertex_buffer.buffer), std::slice::from_ref(&0u64));
            device.cmd_draw(frame_data.command_buffer, cube::CUBE_VERTICES.len() as u32, 1, 0, 0);
        }
    }
}

fn update_global_buffer(frame_data: &FrameData, camera: &Camera) {
    let view_proj = camera.to_view_proj();
    let buffer_data: &[u8] = bytemuck::cast_slice(std::slice::from_ref(&view_proj));
    frame_data.global_data.write_data(buffer_data);
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

fn bind_material_pipeline(device: &Device, swapchain: &Swapchain, pipeline: &MaterialPipeline, frame_data: &FrameData) {
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
}

fn bind_material(device: &Device, frame_data: &FrameData, pipeline: &MaterialPipeline, material: &PbrMaterial, light_data: &LightingDataManager, environment_maps: &EnvironmentMaps) {
    unsafe {
        device.cmd_bind_descriptor_sets(frame_data.command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline_layout, 0, &[frame_data.global_descriptor, material.descriptor_set(), light_data.descriptor_set, environment_maps.irradiance_map_texture.descriptor_set], &[]);
    }
}

fn bind_model(device: &Device, frame_data: &FrameData, mesh: &Mesh) {
    let buffers = &[mesh.vertex_buffer.buffer];
    let offsets = &[0u64];
    unsafe {
        device.cmd_bind_vertex_buffers(frame_data.command_buffer, 0, buffers, offsets);
        device.cmd_bind_index_buffer(frame_data.command_buffer, mesh.index_buffer.buffer, 0, vk::IndexType::UINT32);
    }
}

fn draw_object(device: &Device, frame_data: &FrameData, pipeline: &MaterialPipeline, mesh: &Mesh, world_transform: Mat4) {
    let model_matrix = world_transform * mesh.relative_transform;

    let push_constant = ModelPushConstants {
        model_matrix,
        normal_matrix: model_matrix.inverse().transpose(),
    };
    let model_data: &[u8] = bytemuck::cast_slice(std::slice::from_ref(&push_constant));
    unsafe {
        device.cmd_push_constants(frame_data.command_buffer, pipeline.pipeline_layout, vk::ShaderStageFlags::VERTEX, 0, &[model_data].concat());
        device.cmd_draw_indexed(frame_data.command_buffer, mesh.index_count, 1, 0, 0, 0);
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
        layer_count: 1,
    });
    let clear_color = vk::ClearValue {
        color: vk::ClearColorValue {
            float32: [0.52, 0.8, 0.92, 1.0]
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
        layer_count: 1,
    });
}

// initialisation
impl FrameRenderContext {
    pub fn create(device: ConstPtr<Device>, command_pool: &CommandPool, descriptor_manager: &mut DescriptorManager) -> FrameRenderContext {
        let command_buffers = command_pool.allocate_command_buffers(MAX_FRAMES_IN_FLIGHT as u32);
        let frame_data: [FrameData; MAX_FRAMES_IN_FLIGHT] = (0..MAX_FRAMES_IN_FLIGHT).map(|i| {
            let image_available_semaphore = unsafe { device.create_semaphore(&vkinit::SEMAPHORE_CREATE_INFO, None) }
                .expect("Failed to create semaphore");
            let render_finished_semaphore = unsafe { device.create_semaphore(&vkinit::SEMAPHORE_CREATE_INFO, None) }
                .expect("Failed to create semaphore");
            let in_flight_fence = unsafe { device.create_fence(&vkinit::SIGNALED_FENCE_CREATE_INFO, None) }
                .expect("Failed to create fence");

            let camera_buffer = HostMappedBuffer::create(device, HostMappedBufferCreateInfo {
                size: size_of::<ViewProjectionMatrices>() as u64,
                usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
            });
            let descriptor_buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(camera_buffer.vk_buffer())
                .offset(0)
                .range(size_of::<ViewProjectionMatrices>() as u64);
            let (descriptor_set, _) = descriptor_manager.descriptor_builder()
                .bind_buffer(0, descriptor_buffer_info, vk::DescriptorType::UNIFORM_BUFFER, vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                .build()
                .expect("Failed to build camera descriptor");
            FrameData {
                image_available_semaphore,
                render_finished_semaphore,
                in_flight_fence,
                global_data: camera_buffer,
                command_buffer: command_buffers[i],
                global_descriptor: descriptor_set,
            }
        })
            .collect::<Vec<FrameData>>()
            .try_into()
            .unwrap();

        FrameRenderContext {
            device,
            frame_data,
            current_frame: 0,
        }
    }
}

impl Drop for FrameRenderContext {
    fn drop(&mut self) {
        unsafe {
            for frame_data in &self.frame_data {
                self.device.destroy_semaphore(frame_data.render_finished_semaphore, None);
                self.device.destroy_semaphore(frame_data.image_available_semaphore, None);
                self.device.destroy_fence(frame_data.in_flight_fence, None);
            }
        }
    }
}
