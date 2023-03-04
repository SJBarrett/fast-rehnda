use std::sync::Arc;

use log::info;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::event::WindowEvent;
use winit::event_loop::EventLoopWindowTarget;

use crate::etna;
use crate::etna::{CommandPool, Device, PhysicalDevice, Swapchain, SwapchainError};
use crate::etna::material_pipeline::DescriptorManager;
use crate::rehnda_core::{LongLivedObject, Mat4};
use crate::scene::{Scene, scene_builder};
use crate::ui::UiRunner;

pub struct EtnaEngine {
    // sync objects above here
    scene: Scene,
    command_pool: CommandPool,
    frame_renderer: etna::FrameRenderer,
    ui: UiRunner,
    descriptor_manager: DescriptorManager,
    swapchain: Swapchain,
    surface: etna::Surface,
    physical_device: LongLivedObject<PhysicalDevice>,
    device: LongLivedObject<Device>,
    _instance: LongLivedObject<etna::Instance>,
    _entry: ash::Entry,
    window: Arc<winit::window::Window>,
}

impl EtnaEngine {
    pub fn new(window: Arc<winit::window::Window>, event_loop: &EventLoopWindowTarget<()>) -> EtnaEngine {
        let entry = ash::Entry::linked();
        let instance = LongLivedObject::new(etna::Instance::new(&entry));
        let surface = etna::Surface::new(&entry, &instance, window.raw_display_handle(), window.raw_window_handle()).expect("Failed to create surface");
        let physical_device = LongLivedObject::new(PhysicalDevice::pick_physical_device(instance.ptr(), &surface));
        info!("Graphics Settings: {:?}", physical_device.graphics_settings);
        let device = LongLivedObject::new(Device::create(&instance, &surface, &physical_device));
        let command_pool = CommandPool::create(device.ptr(), physical_device.queue_families().graphics_family);
        let swapchain = Swapchain::create(
            &instance,
            device.ptr(),
            &physical_device,
            &surface,
            &command_pool,
            &physical_device.queue_families(),
            surface.query_best_swapchain_creation_details(window.inner_size(), physical_device.handle()),
        );
        let mut descriptor_manager = DescriptorManager::create(device.ptr());
        let ui = UiRunner::create(device.ptr(), event_loop, &physical_device.graphics_settings, &swapchain);
        let scene = scene_builder::basic_scene(device.ptr(), physical_device.ptr(), &swapchain, &mut descriptor_manager);

        let frame_renderer = etna::FrameRenderer::create(device.ptr(), &command_pool, &mut descriptor_manager);

        EtnaEngine {
            window,
            _entry: entry,
            _instance: instance,
            descriptor_manager,
            ui,
            surface,
            physical_device,
            device,
            swapchain,
            frame_renderer,
            scene,
            command_pool,
        }
    }


    pub fn render(&mut self) {
        if self.is_minimized() {
            return;
        }
        self.ui.update_ui_state(&self.window);
        Self::update_scene(&mut self.scene);
        let draw_result = self.frame_renderer.draw_frame(&self.physical_device, &self.command_pool, &self.swapchain, &self.scene, &mut self.ui.egui_renderer);
        match draw_result {
            Ok(_) => {}
            Err(SwapchainError::RequiresRecreation) => {
                if self.is_minimized() {
                    return;
                }
                self.swapchain.recreate(
                    &self.physical_device,
                    &self.surface,
                    &self.command_pool,
                    &self.physical_device.queue_families(),
                    self.surface.query_best_swapchain_creation_details(self.window.inner_size(), self.physical_device.handle()),
                );
                self.scene.camera.update_aspect_ratio(self.swapchain.aspect_ratio());
            }
        }
    }

    pub fn handle_window_event(&mut self, window_event: &WindowEvent) {
        self.ui.handle_window_event(window_event);
    }

    fn update_scene(scene: &mut Scene) {
        let delta = scene.delta();
        scene.objects_mut()[0].transform *= Mat4::from_rotation_z(delta * 10.0f32.to_radians());
        scene.end_frame();
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