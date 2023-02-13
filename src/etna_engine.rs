use std::sync::Arc;
use ash::vk;
use log::info;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use crate::etna;

pub struct EtnaEngine {
    // sync objects above here
    frame_renderer: etna::FrameRenderer,
    pipeline: etna::Pipeline,
    swapchain: etna::Swapchain,
    surface: etna::Surface,
    physical_device: etna::PhysicalDevice,
    device: Arc<etna::Device>,
    instance: etna::Instance,
    entry: ash::Entry,
}

impl EtnaEngine {
    pub fn new(window: &winit::window::Window) -> EtnaEngine {
        let entry = ash::Entry::linked();
        let instance = etna::Instance::new(&entry);
        let surface = etna::Surface::new(&entry, &instance, window.raw_display_handle(), window.raw_window_handle()).expect("Failed to create surface");
        let physical_device = etna::PhysicalDevice::pick_physical_device(&instance, &surface);
        let device = Arc::new(etna::Device::create(&instance, &surface, physical_device.vk()));
        let swapchain = etna::Swapchain::create(&instance, device.clone(), &surface, &physical_device.queue_families(), &surface.query_best_swapchain_creation_details(&window, physical_device.vk()));
        let pipeline = etna::Pipeline::new(device.clone(), &swapchain);
        let frame_renderer = etna::FrameRenderer::create(device.clone(), &physical_device.queue_families());


        EtnaEngine {
            entry,
            instance,
            surface,
            physical_device,
            device,
            swapchain,
            pipeline,
            frame_renderer,
        }
    }



    pub fn render(&self) {
        self.frame_renderer.draw_frame(&self.swapchain, &self.pipeline);
    }

    pub fn wait_idle(&self) {
        info!("Waiting for device idle");
        unsafe { self.device.device_wait_idle() }
            .expect("Failed to wait for the device to be idle");
    }
}