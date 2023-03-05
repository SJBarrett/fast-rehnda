use std::collections::HashSet;
use std::ffi::CStr;
use std::ops::Deref;

use ash::extensions::khr;
use ash::vk;
use bevy_ecs::prelude::Res;
use bevy_ecs::system::Resource;

use crate::rehnda_core::{ConstPtr, LongLivedObject};
use crate::etna;
use crate::etna::{GraphicsSettings, MsaaSamples};
use crate::etna::utility::vk_cstr_to_string;

pub const DEVICE_EXTENSIONS: [&CStr; 4] = [
    khr::Swapchain::name(),
    khr::DynamicRendering::name(),
    khr::Synchronization2::name(),
    khr::BufferDeviceAddress::name(),
];

pub type PhysicalDeviceRes<'w> = Res<'w, LongLivedObject<PhysicalDevice>>;


#[derive(Resource)]
pub struct PhysicalDevice {
    instance: ConstPtr<etna::Instance>,
    physical_device: vk::PhysicalDevice,
    pub device_properties: vk::PhysicalDeviceProperties,
    pub supported_features: vk::PhysicalDeviceFeatures,
    pub graphics_settings: GraphicsSettings,
    queue_family_indices: QueueFamilyIndices,
}

impl Deref for PhysicalDevice {
    type Target = vk::PhysicalDevice;

    fn deref(&self) -> &Self::Target {
        &self.physical_device
    }
}

impl PhysicalDevice {
    pub fn handle(&self) -> vk::PhysicalDevice {
        self.physical_device
    }

    pub fn queue_families(&self) -> QueueFamilyIndices {
        self.queue_family_indices
    }

    pub fn pick_physical_device(instance: ConstPtr<etna::Instance>, surface: &etna::Surface) -> PhysicalDevice {
        let physical_devices = unsafe { instance.enumerate_physical_devices() }
            .expect("Couldn't enumerate physical devices");
        if physical_devices.is_empty() {
            panic!("Failed to find GPUs with Vulkan support!");
        }

        let picked_device = physical_devices.into_iter()
            .max_by_key(|device| Self::rate_device_suitability(&instance, surface, *device))
            .expect("Failed to find suitable physical device");
        let chosen_queue_family_indices = instance.find_queue_families(surface, picked_device);
        let device_properties = unsafe { instance.get_physical_device_properties(picked_device) };
        let supported_features = unsafe { instance.get_physical_device_features(picked_device) };
        let graphical_settings = Self::determine_graphical_settings(&device_properties);
        PhysicalDevice {
            instance,
            physical_device: picked_device,
            device_properties,
            supported_features,
            graphics_settings: graphical_settings,
            queue_family_indices: chosen_queue_family_indices.unwrap(),
        }
    }

    pub fn determine_graphical_settings(device_properties: &vk::PhysicalDeviceProperties) -> GraphicsSettings {
        let counts = device_properties.limits.framebuffer_color_sample_counts & device_properties.limits.framebuffer_depth_sample_counts;
        let msaa_samples = if counts.contains(vk::SampleCountFlags::TYPE_64) {
            MsaaSamples::X64
        } else if counts.contains(vk::SampleCountFlags::TYPE_32) {
            MsaaSamples::X32
        } else if counts.contains(vk::SampleCountFlags::TYPE_16) {
            MsaaSamples::X16
        } else if counts.contains(vk::SampleCountFlags::TYPE_8) {
            MsaaSamples::X8
        } else if counts.contains(vk::SampleCountFlags::TYPE_4) {
            MsaaSamples::X4
        } else if counts.contains(vk::SampleCountFlags::TYPE_2) {
            MsaaSamples::X2
        } else {
            MsaaSamples::X1
        };

        GraphicsSettings {
            msaa_samples,
            sample_rate_shading_enabled: false,
        }
    }

    pub fn _find_memory_type(&self, type_filter: u32, properties: vk::MemoryPropertyFlags) -> u32 {
        let memory_properties = unsafe { self.instance.get_physical_device_memory_properties(self.physical_device) };
        for i in 0..memory_properties.memory_type_count {
            if (type_filter & (1u32 << i)) > 0 && memory_properties.memory_types[i as usize].property_flags.contains(properties) {
                return i;
            }
        }
        panic!("Failed to find suitable memory");
    }

    pub fn find_supported_format(&self, candidates: &[vk::Format], tiling: vk::ImageTiling, features: vk::FormatFeatureFlags) -> Option<vk::Format> {
        for candidate in candidates {
            let format_props = unsafe { self.instance.get_physical_device_format_properties(self.physical_device, *candidate) };
            if tiling == vk::ImageTiling::LINEAR && (format_props.linear_tiling_features & features) == features {
                return Some(*candidate);
            } else if tiling == vk::ImageTiling::OPTIMAL && (format_props.optimal_tiling_features & features) == features {
                return Some(*candidate);
            }
        }
        None
    }

    pub fn get_format_properties(&self, format: vk::Format) -> vk::FormatProperties {
        unsafe { self.instance.get_physical_device_format_properties(self.physical_device, format) }
    }

    fn rate_device_suitability(instance: &etna::Instance, surface: &etna::Surface, physical_device: vk::PhysicalDevice) -> Option<usize> {
        let properties = unsafe { instance.get_physical_device_properties(physical_device) };
        let features = unsafe { instance.get_physical_device_features(physical_device) };

        if features.geometry_shader != 1 {
            return None;
        }

        let mut score = 0usize;

        // preference discrete GPUs
        if properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
            score += 1000;
        }
        score += properties.limits.max_image_dimension2_d as usize;

        if features.sampler_anisotropy == vk::TRUE {
            score += 100;
        }

        // are our required device queue type supported?
        let queue_family_indices = instance.find_queue_families(surface, physical_device);
        if !queue_family_indices.is_complete() {
            return None
        }

        // are our required device extensions supported?
        if !Self::does_device_support_required_extensions(instance, physical_device) {
            return None
        }

        // is there adequate swapchain support?
        let swapchain_support = surface.query_swapchain_support_details(physical_device);
        if swapchain_support.formats.is_empty() || swapchain_support.present_modes.is_empty() {
            return None
        }

        Some(score)
    }

    fn does_device_support_required_extensions(instance: &etna::Instance, physical_device: vk::PhysicalDevice) -> bool {
        let mut extension_names = DEVICE_EXTENSIONS.iter()
            .map(|extension_name| extension_name.to_str().unwrap())
            .collect::<HashSet<_>>();
        let device_extension_properties = unsafe { instance.enumerate_device_extension_properties(physical_device) }
            .unwrap();
        for extension in device_extension_properties {
            let available_extension_name = vk_cstr_to_string(extension.extension_name.as_slice());
            extension_names.remove(available_extension_name.as_str());
        }

        extension_names.is_empty()
    }


}

#[derive(Debug, Copy, Clone)]
pub struct QueueFamilyIndices {
    pub graphics_family: u32,
    pub present_family: u32,
}

pub struct PotentialQueueFamilyIndices {
    pub graphics_family: Option<u32>,
    pub present_family: Option<u32>,
}

impl PotentialQueueFamilyIndices {
    pub fn is_complete(&self) -> bool {
        self.graphics_family.is_some() && self.present_family.is_some()
    }

    pub fn unwrap(&self) -> QueueFamilyIndices {
        QueueFamilyIndices {
            graphics_family: self.graphics_family.expect("No graphics family chosen"),
            present_family: self.present_family.expect("No present family chosen"),
        }
    }
}