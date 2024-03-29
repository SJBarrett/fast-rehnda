use std::path::Path;

use bevy_ecs::prelude::*;
use bevy_ecs::system::EntityCommands;
use bevy_hierarchy::{BuildChildren};
use enumflags2::BitFlag;
use glam::{EulerRot, Quat};

use crate::etna::{material_pipeline, Swapchain};
use crate::etna::material_pipeline::DescriptorManager;
use crate::rehnda_core::{ColorRgbaF, Vec3};
use crate::assets::{AssetManager, Camera, skybox};
use crate::assets::light_source::PointLight;
use crate::assets::material_server::{MaterialServer, Shader};
use crate::assets::render_object::{PbrMaterialFeatureFlags, PbrMaterialOptions, PbrMaterialUniforms, RenderObject, Transform};
use crate::assets::skybox::SkyBox;

#[derive(Component)]
pub struct Actor {
    pub name: String,
}

#[derive(Component)]
pub struct ShouldDrawDebug;

pub fn spheres_scene(mut commands: Commands, swapchain: Res<Swapchain>, mut asset_manager: ResMut<AssetManager>, mut material_server: ResMut<MaterialServer>, mut descriptor_manager: ResMut<DescriptorManager>) {
    let mut camera = Camera::new(45.0, swapchain.aspect_ratio(), 0.1, 1000.0);
    camera.position = (1.5, -0.6, 9.7).into();
    camera.yaw = -97.0;
    commands.insert_resource(camera);

    let pbr_material = material_server.load_material(material_pipeline::textured_pipeline, Shader::Pbr);
    let unlit_material = material_server.load_material(material_pipeline::textured_pipeline, Shader::Unlit);
    let skybox_material = material_server.load_material(skybox::skybox_pipeline, Shader::SkyBox);
    let sphere_model = asset_manager.load_gltf(Path::new("assets/models/Sphere/UvSphere.glb"), &mut descriptor_manager, pbr_material)[0];
    asset_manager.load_global_light_map(Path::new("assets/drakensberg_solitary_mountain_8k.hdr"), &mut descriptor_manager, skybox_material);

    for x_index in 0..5 {
        for y_index in 0..2 {
            let roughness = 0.25 * x_index as f32;
            let metallic = 1.0 * y_index as f32;
            let new_material = asset_manager.duplicate_material_with_uniforms(&sphere_model.material_instance_handle, &mut descriptor_manager, &PbrMaterialOptions {
                base_color: ColorRgbaF::new(0.7, 0.1, 0.1, 1.0),
                roughness,
                metallic,
                features: PbrMaterialFeatureFlags::empty(),
            });
            let mut sphere_object = sphere_model;
            sphere_object.material_instance_handle = new_material;
            let sphere = commands.spawn((
                Actor {
                    name: format!("Sphere [R: {:.1}][M: {:.1}]", roughness, metallic),
                },
                Transform {
                    translation: ((x_index as f32 - 2.0), -(y_index as f32 - 0.5), 0.0).into(),
                    rotation: Quat::IDENTITY,
                    scale: Vec3::splat(0.3),
                },
            ));
            add_model_to_parent(sphere, std::slice::from_ref(&sphere_object));
        }
    }

    let flight_helmet = asset_manager.load_gltf(Path::new("../glTF-Sample-Models/2.0/FlightHelmet/glTF/FlightHelmet.glb"), &mut descriptor_manager, pbr_material);
    add_model_to_parent(commands.spawn((
        Actor {
            name: "FlightHelmet".into(),
        },
        Transform {
            translation: (3.5, -1.15, 0.0).into(),
            rotation: Quat::IDENTITY,
            scale: Vec3::splat(4.0),
        },
        ShouldDrawDebug,
    )), flight_helmet.as_slice(),
    );

    let floor = asset_manager.load_gltf(Path::new("../assets/Floor/floor_material.glb"), &mut descriptor_manager, pbr_material);
    add_model_to_parent(commands.spawn((
        Actor {
            name: "Floor".into(),
        },
        Transform {
            translation: (0.0, -1.75, 0.0).into(),
            rotation: Quat::IDENTITY,
            scale: Vec3::splat(4.0),
        },
        ShouldDrawDebug,
    )), floor.as_slice(),
    );

    let water_bottle = asset_manager.load_gltf(Path::new("../glTF-Sample-Models/2.0/WaterBottle/glTF-Binary/WaterBottle.glb"), &mut descriptor_manager, pbr_material);
    add_model_to_parent(commands.spawn((
        Actor {
            name: "WaterBottle".into(),
        },
        Transform {
            translation: (-3.5, 0.15, 0.0).into(),
            rotation: Quat::IDENTITY,
            scale: Vec3::splat(10.0),
        },
        ShouldDrawDebug,
    )), water_bottle.as_slice(),
    );

    let light_bulb_model = asset_manager.load_gltf(Path::new("../glTF-Sample-Models/2.0/WaterBottle/glTF-Binary/WaterBottle.glb"), &mut descriptor_manager, unlit_material);
    let light_bulb_entity = commands.spawn((
        Actor {
            name: "Light".into(),
        }, Transform {
            translation: (10.0, 10.0, 10.0).into(),
            rotation: Quat::IDENTITY,
            scale: Vec3::splat(1.0),
        },
        PointLight {
            light_color: (1.0, 1.0, 1.0).into(),
            emissivity: 100.0,
        },
        ShouldDrawDebug,
    ));
    add_model_to_parent(light_bulb_entity, light_bulb_model.as_slice());

    // commands.spawn(SkyBox {
    //     pipeline: skybox_material,
    //     descriptor_set: ,
    // })
}

pub fn shader_development_scene(mut commands: Commands, swapchain: Res<Swapchain>, mut asset_manager: ResMut<AssetManager>, mut material_server: ResMut<MaterialServer>, mut descriptor_manager: ResMut<DescriptorManager>) {
    let mut camera = Camera::new(45.0, swapchain.aspect_ratio(), 0.1, 1000.0);
    camera.position = (1.5, -0.6, 9.7).into();
    camera.yaw = -97.0;
    commands.insert_resource(camera);

    let pbr_pipeline = material_server.load_material(material_pipeline::textured_pipeline, Shader::Pbr);
    let unlit_material = material_server.load_material(material_pipeline::textured_pipeline, Shader::Unlit);

    let cannon_model = asset_manager.load_gltf(Path::new("../glTF-Sample-Models/2.0/SciFiHelmet/glTF/SciFiHelmet.gltf"), &mut descriptor_manager, pbr_pipeline);
    let light_bulb_model = asset_manager.load_gltf(Path::new("../glTF-Sample-Models/2.0/WaterBottle/glTF-Binary/WaterBottle.glb"), &mut descriptor_manager, unlit_material);

    let cannon_entity = commands.spawn((
        Actor {
            name: "Flight Helmet".into(),
        },
        Transform {
            translation: (0.0, -1.5, 0.0).into(),
            rotation: Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, 0.0),
            scale: Vec3::splat(10.0),
        },
        ShouldDrawDebug,
    ));
    add_model_to_parent(cannon_entity, cannon_model.as_slice());

    let light_bulb_entity = commands.spawn((
        Actor {
            name: "Light".into(),
        }, Transform {
            translation: (2.0, 2.0, 2.0).into(),
            rotation: Quat::IDENTITY,
            scale: Vec3::splat(6.0),
        },
        PointLight {
            light_color: (1.0, 1.0, 1.0).into(),
            emissivity: 100.0,
        },
        ShouldDrawDebug,
    ));
    add_model_to_parent(light_bulb_entity, light_bulb_model.as_slice());
}

fn add_model_to_parent(mut commands1: EntityCommands, cannon_model: &[RenderObject]) {
    commands1.with_children(|parent| {
        for mesh in cannon_model {
            parent.spawn((*mesh, Transform::default()));
        }
    });
}