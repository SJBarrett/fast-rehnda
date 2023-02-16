use std::os::raw::c_void;
use std::sync::Arc;
use ash::vk;
use crate::etna;

pub struct Buffer {
    device: Arc<etna::Device>,
    pub size: u64,
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
            self.device.free_memory(self.memory, None);
        }
    }
}

pub struct BufferCreateInfo {
    pub size: u64,
    pub usage: vk::BufferUsageFlags,
    pub memory_properties: vk::MemoryPropertyFlags,
}

impl Buffer {
    pub fn create_buffer_with_data(device: Arc<etna::Device>, physical_device: &etna::PhysicalDevice, create_info: BufferCreateInfo, data: &[u8]) -> Buffer {
        assert_eq!(create_info.size, data.len() as u64);
        let empty_buffer = Self::create_empty_buffer(device, physical_device, create_info);

        let mapped_memory = unsafe { empty_buffer.device.map_memory(empty_buffer.memory, 0, empty_buffer.size, vk::MemoryMapFlags::empty()) }
            .expect("Failed to map memory");
        unsafe { mapped_memory.copy_from_nonoverlapping(data.as_ptr() as *const c_void, data.len()); }
        unsafe { empty_buffer.device.unmap_memory(empty_buffer.memory); }

        empty_buffer
    }

    pub fn populate_buffer_using_staging_buffer(&mut self, physical_device: &etna::PhysicalDevice, command_pool: &etna::CommandPool, data: &[u8]) {
        let staging_buffer = Self::create_empty_buffer(self.device.clone(), physical_device, BufferCreateInfo {
            size: self.size,
            usage: vk::BufferUsageFlags::TRANSFER_SRC,
            memory_properties: vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        });

        let staging_buffer_memory = unsafe { self.device.map_memory(staging_buffer.memory, 0, self.size, vk::MemoryMapFlags::empty()) }
            .expect("Failed to map memory");
        unsafe { staging_buffer_memory.copy_from_nonoverlapping(data.as_ptr() as *const c_void, data.len()); }
        unsafe { self.device.unmap_memory(staging_buffer.memory); }

        let command_buffer = command_pool.allocate_command_buffers(1)[0];
        unsafe {
            self.device.begin_command_buffer(command_buffer, &vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
                .build()).expect("Failed to begin command buffer for copying to buffer");
            let copy_region = [
                vk::BufferCopy::builder()
                    .size(self.size)
                    .build()
            ];
            self.device.cmd_copy_buffer(command_buffer, staging_buffer.buffer, self.buffer, &copy_region);
            self.device.end_command_buffer(command_buffer).expect("Failed to stop command buffer after copying");
        }
        let command_buffers = &[command_buffer];
        let queue_submit_infos = [
            vk::SubmitInfo::builder()
            .command_buffers(command_buffers)
            .build()
        ];
        unsafe { self.device.queue_submit(self.device.graphics_queue, &queue_submit_infos, vk::Fence::null()) }
            .expect("Failed to submit queue");
        unsafe { self.device.queue_wait_idle(self.device.graphics_queue) }
            .expect("Failed to wait for queue idle");
        unsafe { self.device.free_command_buffers(command_pool.vk(), command_buffers) };
    }

    pub fn create_empty_buffer(device: Arc<etna::Device>, physical_device: &etna::PhysicalDevice, create_info: BufferCreateInfo) -> Buffer {
        let buffer_ci = vk::BufferCreateInfo::builder()
            .size(create_info.size)
            .usage(create_info.usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE); // expected only to be used by a single queue
        let buffer = unsafe { device.create_buffer(&buffer_ci, None) }
            .expect("Failed to create buffer");

        let memory_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
        let memory_type_index = physical_device.find_memory_type(memory_requirements.memory_type_bits, create_info.memory_properties);
        let allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_type_index);

        let memory = unsafe { device.allocate_memory(&allocate_info, None) }
            .expect("Failed to allocate buffer memory");
        unsafe { device.bind_buffer_memory(buffer, memory, 0) }
            .expect("Failed to bind buffer memory");
        Buffer {
            device,
            size: create_info.size,
            buffer,
            memory,
        }
    }
}