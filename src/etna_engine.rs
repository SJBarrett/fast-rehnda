use std::mem::size_of;
use std::sync::Arc;
use ash::vk;
use log::{debug, info};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use crate::etna;
use crate::etna::{BufferCreateInfo, SwapchainError};
use crate::model::{TRIANGLE_VERTICES, Vertex};

pub struct EtnaEngine {
    // sync objects above here
    vertex_buffer: etna::Buffer,
    frame_renderer: etna::FrameRenderer,
    pipeline: etna::Pipeline,
    swapchain: etna::Swapchain,
    surface: etna::Surface,
    physical_device: etna::PhysicalDevice,
    device: Arc<etna::Device>,
    instance: Arc<etna::Instance>,
    entry: ash::Entry,
    window: Arc<winit::window::Window>,
}

impl EtnaEngine {
    pub fn new(window: Arc<winit::window::Window>) -> EtnaEngine {
        let entry = ash::Entry::linked();
        let instance = Arc::new(etna::Instance::new(&entry));
        let surface = etna::Surface::new(&entry, &instance, window.raw_display_handle(), window.raw_window_handle()).expect("Failed to create surface");
        let physical_device = etna::PhysicalDevice::pick_physical_device(instance.clone(), &surface);
        let device = Arc::new(etna::Device::create(&instance, &surface, physical_device.vk()));
        let swapchain = etna::Swapchain::create(
            &instance,
            device.clone(),
            &surface,
            &physical_device.queue_families(),
            surface.query_best_swapchain_creation_details(window.inner_size(), physical_device.vk()),
        );
        let pipeline = etna::Pipeline::new(device.clone(), &swapchain);
        let frame_renderer = etna::FrameRenderer::create(device.clone(), &physical_device.queue_families());

        let buffer_data: &[u8] = bytemuck::cast_slice(&TRIANGLE_VERTICES);
        let vertex_buffer = etna::Buffer::create(
            device.clone(),
            &physical_device,
            BufferCreateInfo {
                size: (size_of::<Vertex>() * TRIANGLE_VERTICES.len()) as u64,
                usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                data: buffer_data,
            }
        );


        EtnaEngine {
            window,
            entry,
            instance,
            surface,
            physical_device,
            device,
            swapchain,
            pipeline,
            frame_renderer,
            vertex_buffer,
        }
    }


    pub fn render(&mut self) {
        if self.is_minimized() {
            return;
        }

        let draw_result = self.frame_renderer.draw_frame(&self.swapchain, &self.pipeline, &self.vertex_buffer);
        match draw_result {
            Ok(_) => {}
            Err(SwapchainError::RequiresRecreation) => {
                if self.is_minimized() {
                    return;
                }
                self.swapchain.recreate(
                    &self.surface,
                    &self.physical_device.queue_families(),
                    self.surface.query_best_swapchain_creation_details(self.window.inner_size(), self.physical_device.vk()),
                );
            }
        }
    }

    pub fn wait_idle(&self) {
        info!("Waiting for device idle");
        unsafe { self.device.device_wait_idle() }
            .expect("Failed to wait for the device to be idle");
    }

    fn is_minimized(&self) -> bool {
        self.window.inner_size().height == 0 || self.window.inner_size().width == 0
    }
}