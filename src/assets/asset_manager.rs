use std::hash::{Hash, Hasher};
use std::path::Path;

use ahash::AHashMap;
use bevy_ecs::system::Resource;

use crate::etna::{CommandPool, Device, Image, PhysicalDevice};
use crate::etna::material_pipeline::{DescriptorManager};
use crate::rehnda_core::ConstPtr;
use crate::assets::gltf_loader;
use crate::assets::material_server::MaterialPipelineHandle;
use crate::assets::render_object::{MaterialHandle, Mesh, PbrMaterial, PbrMaterialUniforms, RenderObject};
use crate::etna::cube_map::{CubeMap, CubeMapManager, CubeMapTexture, EnvironmentMaps};

pub struct LoadedGltfMesh {
    pub mesh_handle: MeshHandle,
    pub material_handle: MaterialHandle,
}

#[derive(Resource)]
pub struct AssetManager {
    device: ConstPtr<Device>,
    physical_device: ConstPtr<PhysicalDevice>,
    resource_command_pool: CommandPool,
    meshes: AHashMap<MeshHandle, Mesh>,
    materials: AHashMap<MaterialHandle, PbrMaterial>,
    pub cube_map_manager: CubeMapManager,
    pub global_light_map: Option<(EnvironmentMaps, MaterialPipelineHandle)>,
}

impl AssetManager {
    pub fn create(device: ConstPtr<Device>, physical_device: ConstPtr<PhysicalDevice>, descriptor_manager: &mut DescriptorManager, resource_command_pool: CommandPool) -> Self {
        let cube_map_manager = CubeMapManager::create(device, descriptor_manager, &resource_command_pool);
        AssetManager {
            device,
            physical_device,
            resource_command_pool,
            meshes: AHashMap::new(),
            materials: AHashMap::new(),
            cube_map_manager,
            global_light_map: None,
        }
    }

    pub fn load_global_light_map(&mut self, light_map_path: &Path, descriptor_manager: &mut DescriptorManager, pipeline: MaterialPipelineHandle) {
        let img = self.cube_map_manager.create_environment_maps(&self.physical_device, &self.resource_command_pool, descriptor_manager, light_map_path);
        self.global_light_map = Some((img, pipeline));
    }

    pub fn load_gltf(&mut self, gltf_path: &Path, descriptor_manager: &mut DescriptorManager, pipeline: MaterialPipelineHandle) -> Vec<RenderObject> {
        let (meshes, materials, mesh_material_indices) = gltf_loader::load_gltf(self.device, &self.physical_device, &self.resource_command_pool, descriptor_manager, gltf_path);

        let material_handles: Vec<MaterialHandle> = materials.into_iter().map(|material| {
            let material_handle = MaterialHandle::new(self.materials.len() as u32);
            self.materials.insert(material_handle, material);
            material_handle
        }).collect();

        std::iter::zip(meshes.into_iter(), mesh_material_indices.into_iter()).into_iter().map(|(mesh, mesh_material_index)| {
            let mesh_handle = MeshHandle::new(self.meshes.len() as u32);
            self.meshes.insert(mesh_handle, mesh);
            let material_handle = material_handles[mesh_material_index];
            RenderObject {
                mesh_handle,
                material_instance_handle: material_handle,
                material_pipeline_handle: pipeline,
            }
        }).collect()
    }

    pub fn duplicate_material_with_uniforms(&mut self, material: &MaterialHandle, descriptor_manager: &mut DescriptorManager, new_uniforms: PbrMaterialUniforms) -> MaterialHandle {
        let material = self.materials.get(material).unwrap();
        let new_material = material.copy_with_new_uniforms(self.device, &self.resource_command_pool, descriptor_manager, new_uniforms);
        let handle = MaterialHandle::new(self.materials.len() as u32);
        self.materials.insert(handle, new_material);
        handle
    }

    pub fn mesh_ref(&self, mesh_handle: &MeshHandle) -> &Mesh {
        unsafe { self.meshes.get(mesh_handle).unwrap_unchecked() }
    }

    pub fn material_ref(&self, material_handle: &MaterialHandle) -> &PbrMaterial {
        unsafe { self.materials.get(material_handle).unwrap_unchecked() }
    }
}

pub type MeshHandle = AssetHandle<Mesh>;

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