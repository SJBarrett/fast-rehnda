use std::path::Path;

use ahash::AHashMap;
use bevy_ecs::prelude::*;
use winit::event::VirtualKeyCode;

use crate::assets::{AssetHandle, shader_compiler};
use crate::etna::{Device, DeviceRes, GraphicsSettings, PhysicalDeviceRes, Swapchain};
use crate::etna::material_pipeline::{DescriptorManager, MaterialPipeline};
use crate::rehnda_core::ConstPtr;
use crate::rehnda_core::input::InputState;

pub type MaterialPipelineHandle = AssetHandle<MaterialPipeline>;

pub enum Shader {
    Default,
    Gooch,
    Unlit,
    Pbr,
    BlinnPhong,
    SkyBox,
}

impl Shader {
    fn shader_paths(&self) -> (&'static str, &'static str) {
        match self {
            Shader::Default => {
                ("shaders/spirv/shader.vert_spv", "shaders/spirv/shader.frag_spv")
            }
            Shader::Gooch => {
                ("shaders/spirv/shader.vert_spv", "shaders/spirv/gooch.frag_spv")
            }
            Shader::Unlit => {
                ("shaders/spirv/shader.vert_spv", "shaders/spirv/unlit.frag_spv")
            }
            Shader::Pbr => {
                ("shaders/spirv/shader.vert_spv", "shaders/spirv/pbr.frag_spv")
            }
            Shader::BlinnPhong => {
                ("shaders/spirv/shader.vert_spv", "shaders/spirv/blinnphong.frag_spv")
            }
            Shader::SkyBox => {
                ("shaders/spirv/skybox.vert_spv", "shaders/spirv/skybox.frag_spv")
            }
        }
    }
}

struct MaterialAsset {
    materials: [Option<MaterialPipeline>; 2],
    current_material: usize,
    frames_since_pending_deletion: usize,
    material_creation_function: fn(ConstPtr<Device>, &mut DescriptorManager, &GraphicsSettings, &Swapchain, &Path, &Path) -> MaterialPipeline,
    shader: Shader,
}

#[derive(Default, Resource)]
pub struct MaterialServer {
    materials: AHashMap<MaterialPipelineHandle, MaterialAsset>,
}

impl MaterialServer {
    pub fn reload_materials(&mut self) {
        shader_compiler::compile_all_files();
        for (material_handle, material_asset) in self.materials.iter_mut() {
            material_asset.current_material = (material_asset.current_material + 1) % 2;
            material_asset.frames_since_pending_deletion = 1;
        }
    }

    pub fn load_material(&mut self, material_creation_function: fn(ConstPtr<Device>, &mut DescriptorManager, &GraphicsSettings, &Swapchain, &Path, &Path) -> MaterialPipeline, shader: Shader) -> MaterialPipelineHandle {
        let material_handle = MaterialPipelineHandle::new(self.materials.len() as u32);
        self.materials.insert(material_handle, MaterialAsset {
            materials: [None, None],
            current_material: 0,
            material_creation_function,
            frames_since_pending_deletion: 0,
            shader,
        });
        material_handle
    }

    pub fn material_ref(&self, handle: &MaterialPipelineHandle) -> Option<&MaterialPipeline> {
        self.materials.get(handle).and_then(|asset| asset.materials[asset.current_material].as_ref())
    }
}

pub fn material_server_system(mut material_server: ResMut<MaterialServer>, input_state: Res<InputState>, device: DeviceRes, mut descriptor_manager: ResMut<DescriptorManager>, physical_device: PhysicalDeviceRes, swapchain: Res<Swapchain>) {
    for (material_handle, material_asset) in material_server.materials.iter_mut() {
        if material_asset.materials[material_asset.current_material].is_none() {
            let shader_files = material_asset.shader.shader_paths();
            let vert_path = Path::new(shader_files.0);
            let frag_path = Path::new(shader_files.1);
            let loaded_material = (material_asset.material_creation_function)(device.ptr(), &mut descriptor_manager, &physical_device.graphics_settings, &swapchain, &vert_path, &frag_path);
            material_asset.materials[material_asset.current_material] = Some(loaded_material);
        }
        // drop the inactive material
        if material_asset.materials[(material_asset.current_material + 1) % 2].is_some() {
            if material_asset.frames_since_pending_deletion == 0 {
                material_asset.materials[(material_asset.current_material + 1) % 2] = None;
            } else {
                material_asset.frames_since_pending_deletion -= 1;
            }
        }
    }
    if input_state.is_just_down(VirtualKeyCode::Semicolon) {
        material_server.reload_materials();
    }
}

pub fn material_startup_system() {
    shader_compiler::compile_all_files();
}
