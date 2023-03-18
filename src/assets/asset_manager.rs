use std::hash::{Hash, Hasher};
use std::path::Path;

use ahash::AHashMap;
use bevy_ecs::system::Resource;

use crate::etna::{CommandPool, Device, PhysicalDevice};
use crate::etna::material_pipeline::{DescriptorManager};
use crate::rehnda_core::ConstPtr;
use crate::assets::gltf_loader;
use crate::assets::render_object::{Mesh, MultiMeshModel};

#[derive(Resource)]
pub struct AssetManager {
    device: ConstPtr<Device>,
    physical_device: ConstPtr<PhysicalDevice>,
    resource_command_pool: CommandPool,
    models: AHashMap<ModelHandle, Vec<MeshHandle>>,
    meshes: AHashMap<MeshHandle, Mesh>,
}

impl AssetManager {
    pub fn create(device: ConstPtr<Device>, physical_device: ConstPtr<PhysicalDevice>, resource_command_pool: CommandPool) -> Self {
        AssetManager {
            device,
            physical_device,
            resource_command_pool,
            models: AHashMap::new(),
            meshes: AHashMap::new(),
        }
    }

    pub fn load_gltf(&mut self, gltf_path: &Path, descriptor_manager: &mut DescriptorManager) -> ModelHandle {
        let model = gltf_loader::load_gltf(self.device, &self.physical_device, &self.resource_command_pool, descriptor_manager, gltf_path);
        let handle = ModelHandle::new(self.models.len() as u32);
        let meshes = self.load_meshes_for_model(model);
        self.models.insert(handle, meshes);
        handle
    }

    fn load_meshes_for_model(&mut self, model: MultiMeshModel) -> Vec<MeshHandle> {
        model.meshes.into_iter().map(|mesh| {
            let mesh_handle = MeshHandle::new(self.meshes.len() as u32);
            self.meshes.insert(mesh_handle, mesh);
            mesh_handle
        }).collect()
    }

    pub fn meshes_ref(&self, model_handle: &ModelHandle) -> &[MeshHandle] {
        unsafe { self.models.get(model_handle).unwrap_unchecked().as_slice() }
    }

    pub fn mesh_ref(&self, mesh_handle: &MeshHandle) -> &Mesh {
        unsafe { self.meshes.get(mesh_handle).unwrap_unchecked() }
    }
}

pub type MeshHandle = AssetHandle<Mesh>;
pub type ModelHandle = AssetHandle<MultiMeshModel>;

#[derive(Debug)]
pub struct AssetHandle<T> {
    handle: u32,
    marker: std::marker::PhantomData<T>,
}

impl<T> AssetHandle<T> {
    pub fn new(handle: u32) -> AssetHandle<T> {
        AssetHandle {
            handle,
            marker: std::marker::PhantomData,
        }
    }

    pub fn null() -> AssetHandle<T> {
        AssetHandle {
            handle: u32::MAX,
            marker: std::marker::PhantomData,
        }
    }

    pub fn is_null(&self) -> bool {
        self.handle == u32::MAX
    }
}

impl<T> Copy for AssetHandle<T> {}

impl<T> Clone for AssetHandle<T> {
    fn clone(&self) -> Self {
        AssetHandle {
            handle: self.handle,
            marker: std::marker::PhantomData,
        }
    }
}

impl<T> Eq for AssetHandle<T> {}

impl<T> PartialEq for AssetHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl<T> Hash for AssetHandle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(self.handle)
    }
}