use bevy_ecs::prelude::*;
use crate::etna::{Buffer, Texture};
use crate::rehnda_core::Mat4;
use crate::scene::{MaterialHandle, Model, ModelHandle};

#[derive(Component)]
pub struct RenderObject {
    pub global_transform: Mat4,
    pub relative_transform: Mat4,
    pub model_handle: ModelHandle,
    pub material_handle: MaterialHandle,
}

impl RenderObject {
    pub fn new_with_transform(transform: Mat4, model_handle: ModelHandle, material_handle: MaterialHandle) -> RenderObject {
        RenderObject {
            global_transform: transform,
            relative_transform: Mat4::IDENTITY,
            model_handle,
            material_handle,
        }
    }
}

#[derive(Component)]
pub struct Actor {
    pub global_transform: Mat4,
}

pub struct MultiMeshModel {
    pub meshes: Vec<Mesh>,
}

impl MultiMeshModel {
    pub fn with_single_mesh(mesh: Mesh) -> Self {
        MultiMeshModel {
            meshes: vec![mesh],
        }
    }
}

pub struct Mesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub texture: Option<Texture>,
    pub index_count: u32,
    pub relative_transform: Mat4,
}