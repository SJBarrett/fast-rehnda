use std::ffi::{c_void, CStr};
use ash::extensions::ext;
use ash::{Entry, vk};
use log::{debug, error, info, warn};

pub struct DebugLayer {
    debug_utils_loader: ext::DebugUtils,
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl DebugLayer {
    pub fn init(entry: &Entry, instance: &ash::Instance) -> DebugLayer {
        let debug_utils_loader = ext::DebugUtils::new(entry, instance);
        let messenger_create_info = DebugLayer::debug_messenger_create_info();
        let debug_messenger = unsafe {
            debug_utils_loader.create_debug_utils_messenger(&messenger_create_info, None)
                .expect("Failed to create debug utils callback")
        };
        DebugLayer {
            debug_utils_loader,
            debug_messenger,
        }
    }

    pub fn debug_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT {
        vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                // vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE |
                // vk::DebugUtilsMessageSeverityFlagsEXT::INFO |
                vk::DebugUtilsMessageSeverityFlagsEXT::WARNING |
                    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL |
                    vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE |
                    vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
            )
            .pfn_user_callback(Some(vulkan_debug_callback))
            .build()
    }
}

impl Drop for DebugLayer {
    fn drop(&mut self) {
        unsafe {
            self.debug_utils_loader.destroy_debug_utils_messenger(self.debug_messenger, None);
        }
    }
}


unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut c_void,
) -> vk::Bool32 {
    let message_type = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "General",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "Performance",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "Validation",
        _ => "Other",
    };
    let message = CStr::from_ptr((*callback_data).p_message);

    // TODO fix warn message logged on creating device that prints
    // "vkGetPhysicalDeviceProperties2KHR: Emulation found unrecognized structure type in pProperties->pNext - this struct will be ignored"
    if message.to_str().unwrap().contains("vkGetPhysicalDeviceProperties2KHR: Emulation found unrecognized structure type in pProperties->pNext - this struct will be ignored") {
        return vk::FALSE
    }
    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => debug!("[VkDebug][{}] {:?}", message_type, message),
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => info!("[VkDebug][{}] {:?}", message_type, message),
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => warn!("[VkDebug][{}] {:?}", message_type, message),
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => error!("[VkDebug][{}] {:?}", message_type, message),
        _ => {}
    }

    vk::FALSE
}