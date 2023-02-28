use std::path::Path;
use glam::{EulerRot, Quat};
use crate::etna::{CommandPool, Device, material_pipeline, PhysicalDevice, Swapchain};
use crate::etna::material_pipeline::DescriptorManager;
use crate::rehnda_core::{ConstPtr, Mat4, Vec3};
use crate::scene::{Camera, Scene};

pub fn basic_scene(device: ConstPtr<Device>, physical_device: ConstPtr<PhysicalDevice>, swapchain: &Swapchain, descriptor_manager: &mut DescriptorManager) -> Scene {
    let mut camera = Camera::new(45.0, swapchain.aspect_ratio(), 0.1, 100.0);
    camera.transform = Mat4::look_at_rh(Vec3::new(0.0, 8.0, 4.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0));
    let mut scene = Scene::create_empty_scene_with_camera(device, physical_device, CommandPool::create(device, physical_device.queue_families().graphics_family), camera);

    // models
    let viking_model_handle = scene.load_textured_model(Path::new("assets/viking_room.obj"), Path::new("assets/viking_room.png"), descriptor_manager);
    let suzanne = scene.load_model(Path::new("assets/suzanne.obj"));

    let textured_material = scene.add_material( material_pipeline::textured_pipeline(device, descriptor_manager, &physical_device.graphics_settings, swapchain));
    let non_textured_material = scene.add_material( material_pipeline::non_textured_pipeline(device, descriptor_manager, &physical_device.graphics_settings, swapchain));

    // objects
    scene.add_object(Mat4::IDENTITY, viking_model_handle, textured_material);
    scene.add_object(Mat4::from_translation(Vec3::new(-3.0, 0.0, 0.0)), viking_model_handle, textured_material);
    scene.add_object(Mat4::from_scale_rotation_translation((0.5, 0.5, 0.5).into(), Quat::from_euler(EulerRot::XYZ, 90.0f32.to_radians(), 180.0f32.to_radians(), 0.0), (3.0, 0.0, 0.0).into()), suzanne, non_textured_material);
    scene
}