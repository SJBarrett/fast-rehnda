use std::sync::Arc;
use ash::vk;
use crate::etna;

pub struct CommandPool {
    device: Arc<etna::Device>,
    command_pool: vk::CommandPool,
}

impl CommandPool {
    pub fn create(device: Arc<etna::Device>, queue_family_index: u32) -> CommandPool {
        let command_pool_ci = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family_index);
        let command_pool = unsafe { device.create_command_pool(&command_pool_ci, None) }
            .expect("Failed to create command pool");

        CommandPool {
            device,
            command_pool,
        }
    }

    pub fn allocate_command_buffers(&self, num_command_buffers: u32) -> Vec<vk::CommandBuffer> {
        let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.command_pool)
            .command_buffer_count(num_command_buffers)
            .level(vk::CommandBufferLevel::PRIMARY);
        unsafe { self.device.allocate_command_buffers(&command_buffer_alloc_info) }
            .expect("Failed to allocation command buffer")
    }

    pub fn vk(&self) -> vk::CommandPool {
        self.command_pool
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_command_pool(self.command_pool, None);
        }
    }
}