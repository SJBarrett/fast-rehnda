use std::ops::Deref;

use ash::vk;
use bevy_ecs::system::Resource;

use crate::rehnda_core::ConstPtr;
use crate::etna;

#[derive(Resource)]
pub struct CommandPool {
    device: ConstPtr<etna::Device>,
    command_pool: vk::CommandPool,
}

impl CommandPool {
    pub fn create(device: ConstPtr<etna::Device>, queue_family_index: u32) -> CommandPool {
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

    pub fn one_time_command_buffer(&self) -> OneTimeCommandBuffer {
        OneTimeCommandBuffer::start(self.device, self.command_pool)
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_command_pool(self.command_pool, None);
        }
    }
}

pub struct OneTimeCommandBuffer {
    device: ConstPtr<etna::Device>,
    command_buffer: vk::CommandBuffer,
    owning_command_pool: vk::CommandPool,
}

impl Deref for OneTimeCommandBuffer {
    type Target = vk::CommandBuffer;

    fn deref(&self) -> &Self::Target {
        &self.command_buffer
    }
}

impl OneTimeCommandBuffer {
    pub fn handle(&self) -> vk::CommandBuffer {
        self.command_buffer
    }

    fn start(device: ConstPtr<etna::Device>, owning_command_pool: vk::CommandPool) -> OneTimeCommandBuffer {
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(owning_command_pool)
            .command_buffer_count(1);

        let command_buffer = unsafe { device.allocate_command_buffers(&alloc_info) }
            .expect("failed to alloc one time command buffer")[0];
        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { device.begin_command_buffer(command_buffer, &begin_info) }
            .expect("Failed to begin one time command buffer");

        OneTimeCommandBuffer {
            device,
            command_buffer,
            owning_command_pool,
        }
    }

    fn end(&mut self) {
        unsafe {
            self.device.end_command_buffer(self.command_buffer)
                .expect("Failed to end one time command buffer");
            let command_buffers = &[self.command_buffer];
            let submit_info = [vk::SubmitInfo::builder()
                .command_buffers(command_buffers)
                .build()];
            self.device.queue_submit(self.device.graphics_queue, &submit_info, vk::Fence::null())
                .expect("Failed to submit one time command buffer to queue");
            self.device.queue_wait_idle(self.device.graphics_queue)
                .expect("failed to wait for graphics queue idle");
            self.device.free_command_buffers(self.owning_command_pool, command_buffers);
        }
    }
}

impl Drop for OneTimeCommandBuffer {
    fn drop(&mut self) {
        self.end();
    }
}