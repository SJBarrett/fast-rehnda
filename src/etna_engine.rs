use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use lazy_static::lazy_static;

use log::info;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use crate::core::{LongLivedObject, Mat4, Vec3};

use crate::etna;
use crate::etna::{pipelines, SwapchainError};
use crate::etna::pipelines::{DescriptorAllocator, DescriptorLayoutCache};
use crate::scene::{Camera, Model, Scene};

lazy_static! {
    static ref RENDERING_START_TIME: Instant = Instant::now();
}

pub struct EtnaEngine {
    // sync objects above here
    scene: Scene,
    command_pool: etna::CommandPool,
    frame_renderer: etna::FrameRenderer,
    pipeline: etna::pipelines::Pipeline,
    descriptor_layout_cache: DescriptorLayoutCache,
    descriptor_allocator: DescriptorAllocator,
    swapchain: etna::Swapchain,
    surface: etna::Surface,
    physical_device: etna::PhysicalDevice,
    device: LongLivedObject<etna::Device>,
    _instance: LongLivedObject<etna::Instance>,
    _entry: ash::Entry,
    window: Arc<winit::window::Window>,
}

impl EtnaEngine {
    pub fn new(window: Arc<winit::window::Window>) -> EtnaEngine {
        let entry = ash::Entry::linked();
        let instance = LongLivedObject::new(etna::Instance::new(&entry));
        let surface = etna::Surface::new(&entry, &instance, window.raw_display_handle(), window.raw_window_handle()).expect("Failed to create surface");
        let physical_device = etna::PhysicalDevice::pick_physical_device(instance.ptr(), &surface);
        info!("Graphics Settings: {:?}", physical_device.graphics_settings);
        let device = LongLivedObject::new(etna::Device::create(&instance, &surface, &physical_device));
        let command_pool = etna::CommandPool::create(device.ptr(), physical_device.queue_families().graphics_family);
        let swapchain = etna::Swapchain::create(
            &instance,
            device.ptr(),
            &physical_device,
            &surface,
            &command_pool,
            &physical_device.queue_families(),
            surface.query_best_swapchain_creation_details(window.inner_size(), physical_device.vk()),
        );
        let mut descriptor_layout_cache = DescriptorLayoutCache::create(device.ptr());
        let mut descriptor_allocator = DescriptorAllocator::create(device.ptr());
        let mut camera = Camera::new(45.0, swapchain.aspect_ratio(), 0.1, 10.0);
        camera.transform = Mat4::look_at_rh(Vec3::new(2.0, 2.0, 2.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0));
        let scene = Scene {
            camera,
            model: Model::load_from_obj(device.ptr(), &physical_device, &command_pool, Path::new("assets/viking_room.obj"), Path::new("assets/viking_room.png"))
        };

        let frame_renderer = etna::FrameRenderer::create(device.ptr(), &physical_device, &command_pool, &mut descriptor_layout_cache, &mut descriptor_allocator);
        let pipeline = pipelines::textured_pipeline(device.ptr(), &mut descriptor_layout_cache, &mut descriptor_allocator, &physical_device.graphics_settings, &swapchain, &scene.model, &frame_renderer.frame_data[0].camera_buffer);

        EtnaEngine {
            window,
            _entry: entry,
            _instance: instance,
            descriptor_allocator,
            descriptor_layout_cache,
            surface,
            physical_device,
            device,
            swapchain,
            pipeline,
            frame_renderer,
            scene,
            command_pool,
        }
    }


    pub fn render(&mut self) {
        if self.is_minimized() {
            return;
        }
        Self::update_scene(&mut self.scene);
        let draw_result = self.frame_renderer.draw_frame(&self.swapchain, &self.pipeline, &self.scene);
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
                    self.surface.query_best_swapchain_creation_details(self.window.inner_size(), self.physical_device.vk()),
                );
                self.scene.camera.update_aspect_ratio(self.swapchain.aspect_ratio());
            }
        }
    }

    fn update_scene(scene: &mut Scene) {
        let seconds_elapsed = RENDERING_START_TIME.elapsed().as_secs_f32();
        scene.model.transform = Mat4::from_rotation_z(seconds_elapsed * 90.0f32.to_radians());
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