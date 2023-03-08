use std::path::Path;

use bevy_ecs::prelude::*;
use glam::{EulerRot, Quat};

use crate::etna::{DeviceRes, material_pipeline, PhysicalDeviceRes, Swapchain};
use crate::etna::material_pipeline::DescriptorManager;
use crate::rehnda_core::{Mat4, Vec3};
use crate::scene::{AssetManager, Camera, RenderObject};

pub fn multi_object_test_scene(mut commands: Commands, swapchain: Res<Swapchain>, mut asset_manager: ResMut<AssetManager>, device: DeviceRes, physical_device: PhysicalDeviceRes, mut descriptor_manager: ResMut<DescriptorManager>) {
    let mut camera = Camera::new(45.0, swapchain.aspect_ratio(), 0.1, 100.0);
    camera.transform = Mat4::look_at_rh(Vec3::new(0.0, 8.0, 4.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0));
    commands.insert_resource(camera);
    let cube_model = asset_manager.load_gltf(Path::new("assets/models/box/Box.gltf"), &mut descriptor_manager);
    let viking_model_handle = asset_manager.load_textured_model(Path::new("assets/viking_room.obj"), Path::new("assets/viking_room.png"), &mut descriptor_manager);
    let suzanne = asset_manager.load_model(Path::new("assets/suzanne.obj"));

    let textured_material = asset_manager.add_material(material_pipeline::textured_pipeline(device.ptr(), &mut descriptor_manager, &physical_device.graphics_settings, &swapchain));
    let non_textured_material = asset_manager.add_material(material_pipeline::non_textured_pipeline(device.ptr(), &mut descriptor_manager, &physical_device.graphics_settings, &swapchain));


    commands.spawn_batch(vec![
        (RenderObject {
            transform: Mat4::IDENTITY,
            model_handle: viking_model_handle,
            material_handle: textured_material,
        }),
        (RenderObject {
            transform: Mat4::from_translation(Vec3::new(-3.0, 0.0, 0.0)),
            model_handle: cube_model,
            material_handle: non_textured_material,
        }),
        (RenderObject {
            transform: Mat4::from_scale_rotation_translation((0.5, 0.5, 0.5).into(), Quat::from_euler(EulerRot::XYZ, 90.0f32.to_radians(), 180.0f32.to_radians(), 0.0), (0.0, 1.0, 0.0).into()),
            model_handle: suzanne,
            material_handle: non_textured_material,
        }),
    ])
}

pub fn gltf_test_scene(mut commands: Commands, swapchain: Res<Swapchain>, mut asset_manager: ResMut<AssetManager>, device: DeviceRes, physical_device: PhysicalDeviceRes, mut descriptor_manager: ResMut<DescriptorManager>) {
    let mut camera = Camera::new(45.0, swapchain.aspect_ratio(), 0.1, 100.0);
    camera.transform = Mat4::look_at_rh(Vec3::new(2.0, 6.0, 2.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0));
    commands.insert_resource(camera);

    let path = Path::new("assets/models/BoxTextured/glTF/BoxTextured.gltf");
    let gltf_model = asset_manager.load_gltf(path, &mut descriptor_manager);
    let gltf_model_2 = asset_manager.load_gltf(Path::new("assets/models/Box/Box.gltf"), &mut descriptor_manager);

    let textured_material = asset_manager.add_material(
        material_pipeline::textured_pipeline(device.ptr(), &mut descriptor_manager, &physical_device.graphics_settings, &swapchain)
    );
    let non_textured_material = asset_manager.add_material(
        material_pipeline::non_textured_pipeline(device.ptr(), &mut descriptor_manager, &physical_device.graphics_settings, &swapchain)
    );


    commands.spawn_batch(vec![
        (RenderObject {
            transform: Mat4::IDENTITY,
            model_handle: gltf_model,
            material_handle: textured_material,
        }),
        // (RenderObject {
        //     transform: Mat4::from_translation((0.0, 2.0, 0.0).into()),
        //     model_handle: gltf_model_2,
        //     material_handle: non_textured_material,
        // })
    ])
}