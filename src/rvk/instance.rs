use std::ffi::CString;
use std::ops::Deref;
use std::os::raw::c_char;

use ash::{Entry, vk};
use ash::extensions::{ext, khr};
use log::info;
use crate::rvk;

use crate::rvk::debug::DebugLayer;
use crate::rvk::utility::vk_cstr_to_string;

pub struct Instance {
    instance: ash::Instance,
    debug_layer: Option<DebugLayer>,
}

#[cfg(debug_assertions)]
pub const VALIDATION_LAYERS: [&str; 1] = [
    "VK_LAYER_KHRONOS_validation"
];
#[cfg(not(debug_assertions))]
pub const VALIDATION_LAYERS: [&str; 0] = [];

impl Deref for Instance {
    type Target = ash::Instance;

    fn deref(&self) -> &Self::Target {
        &self.instance
    }
}

// creation
impl Instance {
    pub fn new(entry: &Entry) -> Instance {
        if !are_desired_validation_layers_supported(entry) {
            panic!("Required validation layers not supported");
        }

        let application_name: CString = CString::new("Fast Rehnda").unwrap();
        let application_version: u32 = vk::make_api_version(0, 0, 1, 0);
        let engine_name: CString = CString::new("Fast Rehnda").unwrap();
        let engine_version: u32 = vk::make_api_version(0, 0, 1, 0);
        // vulkan spec 1.0.0
        let vulkan_api_version: u32 = vk::make_api_version(0, 1, 0, 0);

        let app_info = vk::ApplicationInfo::builder()
            .application_name(&application_name)
            .application_version(application_version)
            .engine_name(&engine_name)
            .engine_version(engine_version)
            .api_version(vulkan_api_version);

        let needed_extensions = entry.enumerate_instance_extension_properties(None)
            .expect("Couldn't enumerate extension properties");
        info!("Supported extensions: {:?}", needed_extensions);

        let required_extension_names = required_extension_names();
        let validation_layer_names = VALIDATION_LAYERS.map(|layer| layer.as_ptr() as *const c_char);
        let mut create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(required_extension_names.as_slice())
            .enabled_layer_names(validation_layer_names.as_slice());

        let mut debug_create_info = DebugLayer::debug_messenger_create_info();
        if cfg!(debug_assertions) {
            create_info = create_info.push_next(&mut debug_create_info);
        }

        let instance = unsafe {
            entry.create_instance(&create_info, None).expect("Failed to create instance")
        };
        let debug_layer = if cfg!(debug_assertions) {
            Some(DebugLayer::init(entry, &instance))
        } else {
            None
        };

        Instance {
            instance,
            debug_layer,
        }
    }
}
// destruction
impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}

// Custom functions in Instance
impl Instance {
    pub fn pick_physical_device(&self, surface: &rvk::Surface) -> vk::PhysicalDevice {
        let physical_devices = unsafe { self.instance.enumerate_physical_devices() }
            .expect("Couldn't enumerate physical devices");
        if physical_devices.is_empty() {
            panic!("Failed to find GPUs with Vulkan support!");
        }

        let picked_device = physical_devices.into_iter()
            .max_by_key(|device| self.rate_device_suitability(surface, *device));

        picked_device.expect("Failed to find suitable physical device")
    }

    pub fn find_queue_families(&self, surface: &rvk::Surface, physical_device: vk::PhysicalDevice) -> QueueFamilyIndices {
        let queue_families = unsafe { self.instance.get_physical_device_queue_family_properties(physical_device) };
        let mut queue_family_indices = QueueFamilyIndices {
            graphics_family: None,
            present_family: None,
        };
        for (index, queue_family) in queue_families.iter().enumerate() {
            if surface.get_physical_device_surface_support(physical_device, index as u32).unwrap() {
                queue_family_indices.present_family = Some(index as u32);
            }
            if queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                queue_family_indices.graphics_family = Some(index as u32);
            }

            if queue_family_indices.is_complete() {
                break;
            }
        }
        queue_family_indices
    }

    fn rate_device_suitability(&self, surface: &rvk::Surface, physical_device: vk::PhysicalDevice) -> Option<usize> {
        let properties = unsafe { self.instance.get_physical_device_properties(physical_device) };
        let features = unsafe { self.instance.get_physical_device_features(physical_device) };

        if features.geometry_shader != 1 {
            return None;
        }

        let mut score = 0usize;

        if properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
            score += 1000;
        }
        score += properties.limits.max_image_dimension2_d as usize;

        let queue_family_indices = self.find_queue_families(surface, physical_device);
        if !queue_family_indices.is_complete() {
            return None
        }

        Some(score)
    }
}

pub struct QueueFamilyIndices {
    pub graphics_family: Option<u32>,
    pub present_family: Option<u32>,
}

impl QueueFamilyIndices {
    pub fn is_complete(&self) -> bool {
        self.graphics_family.is_some() && self.present_family.is_some()
    }
}



fn are_desired_validation_layers_supported(entry: &Entry) -> bool {
    let layer_properties = entry.enumerate_instance_layer_properties().expect("Could enumerate layer properties");
    for layer_name in VALIDATION_LAYERS {
        let layer_found = layer_properties.iter()
            .any(|layer_property| vk_cstr_to_string(&layer_property.layer_name) == layer_name);
        if !layer_found {
            return false;
        }
    }
    info!("All required validation layers are supported");
    true
}


#[cfg(all(windows))]
fn required_extension_names() -> Vec<*const i8> {
    vec![
        khr::Surface::name().as_ptr(),
        khr::Win32Surface::name().as_ptr(),
        #[cfg(debug_assertions)] ext::DebugUtils::name().as_ptr(),
    ]
}
