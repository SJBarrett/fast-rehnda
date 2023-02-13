use std::fs::File;
use std::io::Read;
use std::path::Path;
use ash::vk;

pub fn load_shader_module_from_file(device: &ash::Device, shader_path: &Path) -> vk::ShaderModule {
    let file = File::open(shader_path).expect(&format!("Failed to find spv file at {:?}", shader_path));
    let bytes = file.bytes().filter_map(|byte| byte.ok()).collect::<Vec<u8>>();


    let shader_ci = vk::ShaderModuleCreateInfo {
        code_size: bytes.len(),
        p_code: bytes.as_ptr() as *const u32,
        ..Default::default()
    };
    unsafe { device.create_shader_module(&shader_ci, None) }
        .expect("Failed to create shader module")
}
