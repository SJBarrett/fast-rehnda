use ash::vk;

pub const SEMAPHORE_CREATE_INFO: vk::SemaphoreCreateInfo = vk::SemaphoreCreateInfo {
    s_type: vk::StructureType::SEMAPHORE_CREATE_INFO,
    p_next: std::ptr::null(),
    flags: vk::SemaphoreCreateFlags::empty(),
};


pub const SIGNALED_FENCE_CREATE_INFO: vk::FenceCreateInfo = vk::FenceCreateInfo {
    s_type:vk::StructureType::FENCE_CREATE_INFO,
    p_next: std::ptr::null(),
    flags: vk::FenceCreateFlags::SIGNALED,

};

pub const FENCE_CREATE_INFO: vk::FenceCreateInfo = vk::FenceCreateInfo {
    s_type:vk::StructureType::FENCE_CREATE_INFO,
    p_next: std::ptr::null(),
    flags: vk::FenceCreateFlags::empty(),
};

pub const COMMAND_BUFFER_BEGIN_INFO: vk::CommandBufferBeginInfo = vk::CommandBufferBeginInfo {
    s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
    p_next: std::ptr::null(),
    flags: vk::CommandBufferUsageFlags::empty(),
    p_inheritance_info: std::ptr::null(),
};
