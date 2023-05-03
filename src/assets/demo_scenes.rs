use std::path::Path;

use bevy_ecs::prelude::*;
use bevy_ecs::system::EntityCommands;
use bevy_hierarchy::{BuildChildren, Children};
use glam::{EulerRot, Quat};

use crate::etna::{material_pipeline, Swapchain};
use crate::etna::material_pipeline::DescriptorManager;
use crate::rehnda_core::{Mat4, Vec3};
use crate::assets::{AssetManager, Camera};
use crate::assets::light_source::PointLight;
use crate::assets::material_server::{MaterialServer, Shader};
use crate::assets::render_object::{RenderObject, Transform};

#[derive(Component)]
pub struct Actor {
    pub name: String,
}

pub fn spheres_scene(mut commands: Commands, swapchain: Res<Swapchain>, mut asset_manager: ResMut<AssetManager>, mut material_server: ResMut<MaterialServer>, mut descriptor_manager: ResMut<DescriptorManager>) {
    let mut camera = Camera::new(45.0, swapchain.aspect_ratio(), 0.1, 1000.0);
    camera.position = (1.5, -0.6, 9.7).into();
    camera.yaw = -97.0;
    commands.insert_resource(camera);

    let pbr_material = material_server.load_material(material_pipeline::textured_pipeline, Shader::Pbr);
    let unlit_material = material_server.load_material(material_pipeline::textured_pipeline, Shader::Unlit);
    let sphere_model = asset_manager.load_gltf(Path::new("assets/models/Sphere/UvSphere.glb"), &mut descriptor_manager, pbr_material)[0];

    let sphere_material = asset_manager.material_ref(&sphere_model.material_instance_handle);
}

pub fn shader_development_scene(mut commands: Commands, swapchain: Res<Swapchain>, mut asset_manager: ResMut<AssetManager>, mut material_server: ResMut<MaterialServer>, mut descriptor_manager: ResMut<DescriptorManager>) {
    let mut camera = Camera::new(45.0, swapchain.aspect_ratio(), 0.1, 1000.0);
    camera.position = (1.5, -0.6, 9.7).into();
    camera.yaw = -97.0;
    commands.insert_resource(camera);

    let pbr_pipeline = material_server.load_material(material_pipeline::textured_pipeline, Shader::Pbr);
    let unlit_material = material_server.load_material(material_pipeline::textured_pipeline, Shader::Unlit);

    let cannon_model = asset_manager.load_gltf(Path::new("../glTF-Sample-Models/2.0/Lantern/glTF-Binary/Lantern.glb"), &mut descriptor_manager, pbr_pipeline);
    let light_bulb_model = asset_manager.load_gltf(Path::new("../glTF-Sample-Models/2.0/WaterBottle/glTF-Binary/WaterBottle.glb"), &mut descriptor_manager, unlit_material);

    let cannon_entity = commands.spawn((
        Actor {
            name: "Cannon".into(),
        },
        Transform {
            translation: (0.0, -1.5, 0.0).into(),
            rotation: Quat::from_euler(EulerRot::XYZ, 0.0, 60.0f32.to_radians(), 0.0),
            scale: Vec3::splat(0.1),
        }
    ));
    add_model_to_parent(cannon_entity, cannon_model);

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
        }
    ));
    add_model_to_parent(light_bulb_entity, light_bulb_model);
}

fn add_model_to_parent(mut commands1: EntityCommands, cannon_model: Vec<RenderObject>) {
    commands1.with_children(|parent| {
        for mesh in cannon_model {
            parent.spawn((mesh, Transform::default()));
        }
    });
}