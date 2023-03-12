use std::path::Path;

use bevy_ecs::prelude::*;
use glam::{EulerRot, Quat};

use crate::etna::{DeviceRes, material_pipeline, PhysicalDeviceRes, Swapchain};
use crate::etna::material_pipeline::DescriptorManager;
use crate::rehnda_core::{Mat4, Vec3};
use crate::scene::{AssetManager, Camera};
use crate::scene::render_object::RenderObject;

pub fn multi_object_test_scene(mut commands: Commands, swapchain: Res<Swapchain>, mut asset_manager: ResMut<AssetManager>, device: DeviceRes, physical_device: PhysicalDeviceRes, mut descriptor_manager: ResMut<DescriptorManager>) {
    let mut camera = Camera::new(45.0, swapchain.aspect_ratio(), 0.1, 1000.0);
    // camera.transform = Mat4::look_at_rh(Vec3::new(0.0, 8.0, 4.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0));
    commands.insert_resource(camera);
    // let viking_model_handle = asset_manager.load_textured_model(Path::new("assets/viking_room.obj"), Path::new("assets/viking_room.png"), &mut descriptor_manager);
    // let suzanne = asset_manager.load_model(Path::new("assets/suzanne.obj"));
    //
    // let textured_material = asset_manager.add_material(material_pipeline::textured_pipeline(device.ptr(), &mut descriptor_manager, &physical_device.graphics_settings, &swapchain));
    // let non_textured_material = asset_manager.add_material(material_pipeline::non_textured_pipeline(device.ptr(), &mut descriptor_manager, &physical_device.graphics_settings, &swapchain));
    //
    //
    // commands.spawn_batch(vec![
    //     (RenderObject {
    //         global_transform: Mat4::IDENTITY,
    //         relative_transform: Default::default(),
    //         model_handle: viking_model_handle,
    //         material_handle: textured_material,
    //     }),
    //     (RenderObject {
    //         global_transform: Mat4::from_scale_rotation_translation((0.5, 0.5, 0.5).into(), Quat::from_euler(EulerRot::XYZ, 90.0f32.to_radians(), 180.0f32.to_radians(), 0.0), (0.0, 1.0, 0.0).into()),
    //         relative_transform: Default::default(),
    //         model_handle: suzanne,
    //         material_handle: non_textured_material,
    //     }),
    // ])
}

#[derive(Component)]
pub struct Actor {
    pub name: String,
}

pub fn gltf_test_scene(mut commands: Commands, swapchain: Res<Swapchain>, mut asset_manager: ResMut<AssetManager>, device: DeviceRes, physical_device: PhysicalDeviceRes, mut descriptor_manager: ResMut<DescriptorManager>) {
    let mut camera = Camera::new(45.0, swapchain.aspect_ratio(), 0.1, 1000.0);
    // camera.transform = Mat4::look_at_rh(Vec3::new(-6.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0));
    // camera.transform = Mat4::from_scale_rotation_translation((1.0, 1.0, 1.0).into(), Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, 0.0), (0.0, 0.0, 0.0).into());
    commands.insert_resource(camera);

    let path = Path::new("../glTF-Sample-Models/2.0/Duck/glTF/Duck.gltf");
    let gltf_model = asset_manager.load_gltf(path, &mut descriptor_manager);

    let textured_material = asset_manager.add_material(
        material_pipeline::textured_pipeline(device.ptr(), &mut descriptor_manager, &physical_device.graphics_settings, &swapchain)
    );

    commands.spawn_batch(vec![
        (RenderObject {
            global_transform: Mat4::from_scale_rotation_translation(Vec3::splat(1.0), Quat::IDENTITY, (30.0, 0.0, 0.0).into()),
            relative_transform: Default::default(),
            model_handle: asset_manager.load_gltf(Path::new("../glTF-Sample-Models/2.0/OrientationTest/glTF/OrientationTest.gltf"), &mut descriptor_manager),
            material_handle: textured_material,
        }, Actor { name: "BoomBox".into() }),
        // (RenderObject {
        //     global_transform: Mat4::from_scale_rotation_translation((1.0, 1.0, 1.0).into(), Quat::IDENTITY, (0.0, 0.0, 3.0).into()),
        //     relative_transform: Default::default(),
        //     model_handle: asset_manager.load_gltf(Path::new("../glTF-Sample-Models/2.0/Box With Spaces/glTF/Box With Spaces.gltf"), &mut descriptor_manager),
        //     material_handle: textured_material,
        // }, Actor { name: "BoxTextured".into() }),
        // (RenderObject {
        //     global_transform: Mat4::from_scale_rotation_translation((1.0, 1.0, 1.0).into(), Quat::IDENTITY, (3.0, 0.0, 0.0).into()),
        //     relative_transform: Default::default(),
        //     model_handle: asset_manager.load_gltf(Path::new("assets/models/AntiqueCamera/glTF/AntiqueCamera.gltf"), &mut descriptor_manager),
        //     material_handle: textured_material,
        // },  Actor { name: "AntiqueCamera".into() }),
    ])
}
