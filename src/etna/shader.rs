use std::fs::File;
use std::io::Read;
use std::path::Path;
use ash::vk;
use crate::rehnda_core::ConstPtr;
use crate::etna;

pub struct ShaderModule {
    device: ConstPtr<etna::Device>,
    shader_module: vk::ShaderModule,
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_shader_module(self.shader_module, None);
        }
    }
}

impl ShaderModule {
    pub fn load_from_file(device: ConstPtr<etna::Device>, shader_path: &Path) -> ShaderModule {
        let file = File::open(shader_path).expect(&format!("Failed to find spv file at {:?}", shader_path));
        let bytes = file.bytes().filter_map(|byte| byte.ok()).collect::<Vec<u8>>();


        let shader_ci = vk::ShaderModuleCreateInfo {
            code_size: bytes.len(),
            p_code: bytes.as_ptr() as *const u32,
            ..Default::default()
        };
        let shader_module = unsafe { device.create_shader_module(&shader_ci, None) }
            .expect("Failed to create shader module");
        ShaderModule {
            device,
            shader_module,
        }
    }

    pub fn handle(&self) -> vk::ShaderModule {
        self.shader_module
    }
}
