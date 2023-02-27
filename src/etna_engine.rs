use std::path::Path;
use std::sync::Arc;

use log::info;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use crate::core::{LongLivedObject, Mat4, Vec3};
use crate::etna;
use crate::etna::{CommandPool, material_pipeline, SwapchainError};
use crate::etna::material_pipeline::DescriptorManager;
use crate::scene::{Camera, Scene};



pub struct EtnaEngine {
    // sync objects above here
    scene: Scene,
    command_pool: etna::CommandPool,
    frame_renderer: etna::FrameRenderer,
    descriptor_manager: DescriptorManager,
    swapchain: etna::Swapchain,
    surface: etna::Surface,
    physical_device: LongLivedObject<etna::PhysicalDevice>,
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
        let physical_device = LongLivedObject::new(etna::PhysicalDevice::pick_physical_device(instance.ptr(), &surface));
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
        let mut descriptor_manager = DescriptorManager::create(device.ptr());
        let mut camera = Camera::new(45.0, swapchain.aspect_ratio(), 0.1, 100.0);
        camera.transform = Mat4::look_at_rh(Vec3::new(0.0, 8.0, 4.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0));
        // TODO use a transfer queue instead of graphics
        let mut scene = Scene::create_empty_scene_with_camera(device.ptr(), physical_device.ptr(), CommandPool::create(device.ptr(), physical_device.queue_families().graphics_family), camera);
        let viking_model_handle = scene.load_model(Path::new("assets/viking_room.obj"), Path::new("assets/viking_room.png"));
        let pipeline = material_pipeline::textured_pipeline(device.ptr(), &mut descriptor_manager, &physical_device.graphics_settings, &swapchain, &scene.model_ref(&viking_model_handle));
        let basic_material_handle = scene.add_material(pipeline);
        scene.add_object(Mat4::IDENTITY, viking_model_handle, basic_material_handle);
        scene.add_object(Mat4::from_translation(Vec3::new(-3.0, 0.0, 0.0)), viking_model_handle, basic_material_handle);
        scene.add_object(Mat4::from_translation(Vec3::new(3.0, 0.0, 0.0)) * Mat4::from_rotation_z(90f32.to_radians()), viking_model_handle, basic_material_handle);

        let frame_renderer = etna::FrameRenderer::create(device.ptr(), &physical_device, &command_pool, &mut descriptor_manager);

        EtnaEngine {
            window,
            _entry: entry,
            _instance: instance,
            descriptor_manager,
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
        Self::update_scene(&mut self.scene);
        let draw_result = self.frame_renderer.draw_frame(&self.swapchain, &self.scene);
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
        let delta = scene.delta();
        scene.objects_mut()[0].transform *= Mat4::from_rotation_z(delta * 10.0f32.to_radians());
        scene.objects_mut()[1].transform *= Mat4::from_rotation_z(delta * 20.0f32.to_radians());
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