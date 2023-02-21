use std::path::Path;
use std::sync::Arc;

use log::info;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use crate::etna;
use crate::etna::SwapchainError;
use crate::model::Model;

pub struct EtnaEngine {
    // sync objects above here
    model: Model,
    command_pool: etna::CommandPool,
    frame_renderer: etna::FrameRenderer,
    pipeline: etna::Pipeline,
    swapchain: etna::Swapchain,
    surface: etna::Surface,
    physical_device: etna::PhysicalDevice,
    device: Arc<etna::Device>,
    _instance: Arc<etna::Instance>,
    _entry: ash::Entry,
    window: Arc<winit::window::Window>,
}

impl EtnaEngine {
    pub fn new(window: Arc<winit::window::Window>) -> EtnaEngine {
        let entry = ash::Entry::linked();
        let instance = Arc::new(etna::Instance::new(&entry));
        let surface = etna::Surface::new(&entry, &instance, window.raw_display_handle(), window.raw_window_handle()).expect("Failed to create surface");
        let physical_device = etna::PhysicalDevice::pick_physical_device(instance.clone(), &surface);
        let device = Arc::new(etna::Device::create(&instance, &surface, &physical_device));
        let swapchain = etna::Swapchain::create(
            &instance,
            device.clone(),
            &surface,
            &physical_device.queue_families(),
            surface.query_best_swapchain_creation_details(window.inner_size(), physical_device.vk()),
        );
        let pipeline = etna::Pipeline::new(device.clone(), &swapchain);
        let command_pool = etna::CommandPool::create(device.clone(), physical_device.queue_families().graphics_family);

        let model = Model::load_from_obj(device.clone(), &physical_device, &command_pool, Path::new("assets/viking_room.obj"), Path::new("assets/viking_room.png"));

        let frame_renderer = etna::FrameRenderer::create(device.clone(), &physical_device, &pipeline, &command_pool, swapchain.extent, &model);


        EtnaEngine {
            window,
            _entry: entry,
            _instance: instance,
            surface,
            physical_device,
            device,
            swapchain,
            pipeline,
            frame_renderer,
            model,
            command_pool: command_pool,
        }
    }


    pub fn render(&mut self) {
        if self.is_minimized() {
            return;
        }

        let draw_result = self.frame_renderer.draw_frame(&self.swapchain, &self.pipeline, &self.model);
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
                self.frame_renderer.resize(&self.physical_device, &self.command_pool, self.swapchain.extent);
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