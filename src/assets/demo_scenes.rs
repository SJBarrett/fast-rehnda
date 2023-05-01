use std::path::Path;

use bevy_ecs::prelude::*;
use glam::{EulerRot, Quat};

use crate::etna::{DeviceRes, material_pipeline, PhysicalDeviceRes, Swapchain};
use crate::etna::material_pipeline::DescriptorManager;
use crate::rehnda_core::{Mat4, Vec3};
use crate::assets::{AssetManager, Camera};
use crate::assets::light_source::PointLight;
use crate::assets::material_server::{MaterialServer, Shader};
use crate::assets::render_object::RenderObject;

#[derive(Component)]
pub struct Actor {
    pub name: String,
    pub transform: Mat4,
}

pub fn shader_development_scene(mut commands: Commands, swapchain: Res<Swapchain>, mut asset_manager: ResMut<AssetManager>, mut material_server: ResMut<MaterialServer>, mut descriptor_manager: ResMut<DescriptorManager>) {
    let mut camera = Camera::new(45.0, swapchain.aspect_ratio(), 0.1, 1000.0);
    camera.position = (1.5, -0.6, 9.7).into();
    camera.yaw = -97.0;
    commands.insert_resource(camera);

    let pbr_material = material_server.load_material(material_pipeline::textured_pipeline, Shader::Pbr);
    let unlit_material = material_server.load_material(material_pipeline::textured_pipeline, Shader::Unlit);

    commands.spawn((
        RenderObject {
            relative_transform: Default::default(),
            model_handle: asset_manager.load_gltf(Path::new("../glTF-Sample-Models/2.0/FlightHelmet/glTF/FlightHelmet.glb"), &mut descriptor_manager),
            material_handle: pbr_material,
        },
        Actor {
            transform: Mat4::from_scale_rotation_translation(Vec3::splat(10.0), Quat::from_euler(EulerRot::XYZ, 0.0, 60.0f32.to_radians(), 0.0), (0.0, -1.5, 0.0).into()),
            name: "Cannon".into(),
        }
    ));
    commands.spawn((
        RenderObject {
            relative_transform: Default::default(),
            model_handle: asset_manager.load_gltf(Path::new("../glTF-Sample-Models/2.0/WaterBottle/glTF-Binary/WaterBottle.glb"), &mut descriptor_manager),
            material_handle: unlit_material,
        },
        Actor {
            transform: Mat4::from_scale_rotation_translation(Vec3::splat(6.0), Quat::IDENTITY, (2.0, 2.0, 2.0).into()),
            name: "LightBulb".into(),
        },
        PointLight {
            light_color: (1.0, 1.0, 1.0).into(),
            emissivity: 100.0,
        },
    ));
}

pub fn gltf_test_scene(mut commands: Commands, swapchain: Res<Swapchain>, mut asset_manager: ResMut<AssetManager>, mut material_server: ResMut<MaterialServer>, mut descriptor_manager: ResMut<DescriptorManager>) {
    let mut camera = Camera::new(45.0, swapchain.aspect_ratio(), 0.1, 1000.0);
    commands.insert_resource(camera);

    let path = Path::new("../glTF-Sample-Models/2.0/Duck/glTF/Duck.gltf");
    let gltf_model = asset_manager.load_gltf(path, &mut descriptor_manager);

    let textured_material = material_server.load_material(material_pipeline::textured_pipeline, Shader::Default);

    // commands.spawn_batch(vec![
    //     (RenderObject {
    //         global_transform: Mat4::from_scale_rotation_translation(Vec3::splat(1.0), Quat::IDENTITY, (30.0, 0.0, 0.0).into()),
    //         relative_transform: Default::default(),
    //         model_handle: asset_manager.load_gltf(Path::new("../glTF-Sample-Models/2.0/OrientationTest/glTF/OrientationTest.gltf"), &mut descriptor_manager),
    //         material_handle: textured_material,
    //     }, Actor { name: "BoomBox".into() }),
    //     (RenderObject {
    //         global_transform: Mat4::from_scale_rotation_translation((1.0, 1.0, 1.0).into(), Quat::IDENTITY, (0.0, 0.0, 3.0).into()),
    //         relative_transform: Default::default(),
    //         model_handle: asset_manager.load_gltf(Path::new("../glTF-Sample-Models/2.0/Box With Spaces/glTF/Box With Spaces.gltf"), &mut descriptor_manager),
    //         material_handle: textured_material,
    //     }, Actor { name: "BoxTextured".into() }),
    //     (RenderObject {
    //         global_transform: Mat4::from_scale_rotation_translation((1.0, 1.0, 1.0).into(), Quat::IDENTITY, (3.0, 0.0, 0.0).into()),
    //         relative_transform: Default::default(),
    //         model_handle: asset_manager.load_gltf(Path::new("assets/models/AntiqueCamera/glTF/AntiqueCamera.gltf"), &mut descriptor_manager),
    //         material_handle: textured_material,
    //     }, Actor { name: "AntiqueCamera".into() }),
    // ])
}
