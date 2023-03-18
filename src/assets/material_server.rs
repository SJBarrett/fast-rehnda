use std::mem::swap;
use ahash::AHashMap;
use bevy_ecs::prelude::*;
use image::load;
use log::info;
use winit::event::VirtualKeyCode;

use crate::assets::{AssetHandle, shader_compiler};
use crate::etna::{Device, DeviceRes, GraphicsSettings, PhysicalDeviceRes, Swapchain};
use crate::etna::material_pipeline::{DescriptorManager, MaterialPipeline};
use crate::rehnda_core::ConstPtr;
use crate::rehnda_core::input::InputState;

pub type MaterialHandle = AssetHandle<MaterialPipeline>;

struct MaterialAsset {
    materials: [Option<MaterialPipeline>; 2],
    current_material: usize,
    frames_since_pending_deletion: usize,
    material_creation_function: fn(ConstPtr<Device>, &mut DescriptorManager, &GraphicsSettings, &Swapchain) -> MaterialPipeline,
}

#[derive(Default, Resource)]
pub struct MaterialServer {
    materials: AHashMap<MaterialHandle, MaterialAsset>,
}

impl MaterialServer {
    pub fn reload_materials(&mut self) {
        shader_compiler::compile_all_files();
        for (material_handle, material_asset) in self.materials.iter_mut() {
            material_asset.current_material = (material_asset.current_material + 1) % 2;
            material_asset.frames_since_pending_deletion = 1;
        }
    }

    pub fn load_material(&mut self, material_creation_function: fn(ConstPtr<Device>, &mut DescriptorManager, &GraphicsSettings, &Swapchain) -> MaterialPipeline) -> MaterialHandle {
        let material_handle = MaterialHandle::new(self.materials.len() as u32);
        self.materials.insert(material_handle, MaterialAsset {
            materials: [None, None],
            current_material: 0,
            material_creation_function,
            frames_since_pending_deletion: 0,
        });
        material_handle
    }

    pub fn material_ref(&self, handle: &MaterialHandle) -> Option<&MaterialPipeline> {
        self.materials.get(handle).and_then(|asset| asset.materials[asset.current_material].as_ref())
    }
}

pub fn material_server_system(mut material_server: ResMut<MaterialServer>, input_state: Res<InputState>, device: DeviceRes, mut descriptor_manager: ResMut<DescriptorManager>, physical_device: PhysicalDeviceRes, swapchain: Res<Swapchain>) {
    for (material_handle, material_asset) in material_server.materials.iter_mut() {
        if material_asset.materials[material_asset.current_material].is_none() {
            let loaded_material = (material_asset.material_creation_function)(device.ptr(), &mut descriptor_manager, &physical_device.graphics_settings, &swapchain);
            material_asset.materials[material_asset.current_material] = Some(loaded_material);
            info!("Loaded new material");
        }
        // drop the inactive material
        if material_asset.materials[(material_asset.current_material + 1) % 2].is_some() {
            if material_asset.frames_since_pending_deletion == 0 {
                material_asset.materials[(material_asset.current_material + 1) % 2] = None;
                info!("Destroying old material");
            } else {
                material_asset.frames_since_pending_deletion -= 1;
            }
        }
    }
    if input_state.is_just_down(VirtualKeyCode::Semicolon) {
        material_server.reload_materials();
    }
}