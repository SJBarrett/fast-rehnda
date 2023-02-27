use std::path::Path;
use ahash::AHashMap;
use crate::core::ConstPtr;
use crate::etna::{CommandPool, Device, PhysicalDevice};
use crate::scene::{Camera, Model};

pub struct Scene {
    device: ConstPtr<Device>,
    physical_device: ConstPtr<PhysicalDevice>,
    resource_command_pool: CommandPool,
    pub camera: Camera,
    pub model: Model,
    models: AHashMap<ResourceHandle<Model>, Model>,
}

impl Scene {
    pub fn create_empty_scene_with_camera(device: ConstPtr<Device>, physical_device: ConstPtr<PhysicalDevice>, resource_command_pool: CommandPool, camera: Camera) -> Scene {
        Scene {
            device,
            physical_device,
            model: Model::load_from_obj(device, &physical_device, &resource_command_pool, Path::new("assets/viking_room.obj"), Path::new("assets/viking_room.png")),
            resource_command_pool,
            camera,
            models: AHashMap::new(),
        }
    }

    pub fn load_model(device: ConstPtr<Device>, physical_device: &PhysicalDevice, command_pool: &CommandPool, obj_path: &Path, texture_path: &Path) -> ResourceHandle<Model> {
        todo!()
    }
}

pub struct ResourceHandle<T> {
    handle: u32,
    marker: std::marker::PhantomData<T>
}