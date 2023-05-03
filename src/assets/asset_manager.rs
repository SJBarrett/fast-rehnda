use std::hash::{Hash, Hasher};
use std::path::Path;

use ahash::AHashMap;
use bevy_ecs::system::Resource;

use crate::etna::{CommandPool, Device, PhysicalDevice};
use crate::etna::material_pipeline::{DescriptorManager};
use crate::rehnda_core::ConstPtr;
use crate::assets::gltf_loader;
use crate::assets::render_object::{Material, MaterialHandle, Mesh, MultiMeshModel};

#[derive(Resource)]
pub struct AssetManager {
    device: ConstPtr<Device>,
    physical_device: ConstPtr<PhysicalDevice>,
    resource_command_pool: CommandPool,
    models: AHashMap<ModelHandle, Vec<MeshHandle>>,
    meshes: AHashMap<MeshHandle, Mesh>,
    materials: AHashMap<MaterialHandle, Material>,
}

impl AssetManager {
    pub fn create(device: ConstPtr<Device>, physical_device: ConstPtr<PhysicalDevice>, resource_command_pool: CommandPool) -> Self {
        AssetManager {
            device,
            physical_device,
            resource_command_pool,
            models: AHashMap::new(),
            meshes: AHashMap::new(),
            materials: AHashMap::new(),
        }
    }

    pub fn load_gltf(&mut self, gltf_path: &Path, descriptor_manager: &mut DescriptorManager) -> ModelHandle {
        let (meshes, materials, mesh_material_indices) = gltf_loader::load_gltf(self.device, &self.physical_device, &self.resource_command_pool, descriptor_manager, gltf_path);
        let material_handles: Vec<MaterialHandle> = materials.into_iter().map(|material| {
            let material_handle = MaterialHandle::new(self.materials.len() as u32);
            self.materials.insert(material_handle, material);
            material_handle
        }).collect();
        let mesh_handles: Vec<MeshHandle> = std::iter::zip(meshes.into_iter(), mesh_material_indices.into_iter()).map(|(mut mesh, material_index)| {
            let mesh_handle = MeshHandle::new(self.meshes.len() as u32);
            let material_handle = material_handles[material_index];
            mesh.material_handle = material_handle;
            self.meshes.insert(mesh_handle, mesh);
            mesh_handle
        }).collect();
        let handle = ModelHandle::new(self.models.len() as u32);
        self.models.insert(handle, mesh_handles);
        handle
    }

    pub fn meshes_ref(&self, model_handle: &ModelHandle) -> &[MeshHandle] {
        unsafe { self.models.get(model_handle).unwrap_unchecked().as_slice() }
    }

    pub fn mesh_ref(&self, mesh_handle: &MeshHandle) -> &Mesh {
        unsafe { self.meshes.get(mesh_handle).unwrap_unchecked() }
    }

    pub fn material_ref(&self, material_handle: &MaterialHandle) -> &Material {
        unsafe { self.materials.get(material_handle).unwrap_unchecked() }
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