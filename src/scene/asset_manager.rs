use std::hash::{Hash, Hasher};
use std::path::Path;

use ahash::AHashMap;
use bevy_ecs::system::Resource;

use crate::etna::{CommandPool, Device, PhysicalDevice};
use crate::etna::material_pipeline::{DescriptorManager, MaterialPipeline};
use crate::rehnda_core::ConstPtr;
use crate::scene::Model;

#[derive(Resource)]
pub struct AssetManager {
    device: ConstPtr<Device>,
    physical_device: ConstPtr<PhysicalDevice>,
    resource_command_pool: CommandPool,
    models: AHashMap<ModelHandle, Model>,
    materials: AHashMap<MaterialHandle, MaterialPipeline>,
}

impl AssetManager {
    pub fn create(device: ConstPtr<Device>, physical_device: ConstPtr<PhysicalDevice>, resource_command_pool: CommandPool) -> Self {
        AssetManager {
            device,
            physical_device,
            resource_command_pool,
            models: AHashMap::new(),
            materials: AHashMap::new(),
        }
    }

    pub fn load_textured_model(&mut self, obj_path: &Path, texture_path: &Path, descriptor_manager: &mut DescriptorManager) -> ModelHandle {
        let model = Model::load_textured_obj(self.device, &self.physical_device, &self.resource_command_pool, descriptor_manager, obj_path, texture_path);
        let handle = ModelHandle::new(self.models.len() as u32);
        self.models.insert(handle, model);
        handle
    }

    pub fn load_model(&mut self, obj_path: &Path) -> ModelHandle {
        let model = Model::load_obj(self.device, &self.resource_command_pool, obj_path);
        let handle = ModelHandle::new(self.models.len() as u32);
        self.models.insert(handle, model);
        handle
    }

    pub fn add_material(&mut self, material_pipeline: MaterialPipeline) -> MaterialHandle {
        let handle = MaterialHandle::new(self.materials.len() as u32);
        self.materials.insert(handle, material_pipeline);
        handle
    }

    pub fn model_ref(&self, model_handle: &ModelHandle) -> &Model {
        unsafe { self.models.get(model_handle).unwrap_unchecked() }
    }

    pub fn material_ref(&self, material_handle: &MaterialHandle) -> &MaterialPipeline {
        unsafe { self.materials.get(material_handle).unwrap_unchecked() }
    }
}

pub type ModelHandle = ResourceHandle<Model>;
pub type MaterialHandle = ResourceHandle<MaterialPipeline>;

#[derive(Debug)]
pub struct ResourceHandle<T> {
    handle: u32,
    marker: std::marker::PhantomData<T>,
}

impl<T> ResourceHandle<T> {
    pub fn new(handle: u32) -> ResourceHandle<T> {
        ResourceHandle {
            handle,
            marker: std::marker::PhantomData,
        }
    }

    pub fn null() -> ResourceHandle<T> {
        ResourceHandle {
            handle: u32::MAX,
            marker: std::marker::PhantomData,
        }
    }

    pub fn is_null(&self) -> bool {
        self.handle == u32::MAX
    }
}

impl<T> Copy for ResourceHandle<T> {}

impl<T> Clone for ResourceHandle<T> {
    fn clone(&self) -> Self {
        ResourceHandle {
            handle: self.handle,
            marker: std::marker::PhantomData,
        }
    }
}

impl<T> Eq for ResourceHandle<T> {}

impl<T> PartialEq for ResourceHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl<T> Hash for ResourceHandle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(self.handle)
    }
}