use std::os::raw::c_void;

use ash::vk;

use crate::core::ConstPtr;
use crate::etna;

pub struct Buffer {
    device: ConstPtr<etna::Device>,
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

pub struct BufferCreateInfo<'a> {
    pub data: &'a[u8],
    pub usage: vk::BufferUsageFlags,
    pub memory_properties: vk::MemoryPropertyFlags,
}

impl Buffer {
    pub fn create_buffer_with_data(device: ConstPtr<etna::Device>, physical_device: &etna::PhysicalDevice, create_info: BufferCreateInfo) -> Buffer {
        let empty_buffer = Self::create_empty_buffer(device, physical_device, create_info.data.len() as u64, create_info.usage, create_info.memory_properties);

        let mapped_memory = unsafe { empty_buffer.device.map_memory(empty_buffer.memory, 0, empty_buffer.size, vk::MemoryMapFlags::empty()) }
            .expect("Failed to map memory");
        unsafe { mapped_memory.copy_from_nonoverlapping(create_info.data.as_ptr() as *const c_void, create_info.data.len()); }
        unsafe { empty_buffer.device.unmap_memory(empty_buffer.memory); }

        empty_buffer
    }

    pub fn create_and_initialize_buffer_with_staging_buffer(device: ConstPtr<etna::Device>, physical_device: &etna::PhysicalDevice, command_pool: &etna::CommandPool, create_info: BufferCreateInfo) -> Buffer {
        let mut buffer = Self::create_empty_buffer(device, physical_device, create_info.data.len() as u64, create_info.usage, create_info.memory_properties);
        buffer.populate_buffer_using_staging_buffer(physical_device, command_pool, create_info.data);
        buffer
    }

    fn populate_buffer_using_staging_buffer(&mut self, physical_device: &etna::PhysicalDevice, command_pool: &etna::CommandPool, data: &[u8]) {
        let staging_buffer = Self::create_empty_buffer(
            self.device,
            physical_device,
            self.size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE
        );

        let staging_buffer_memory = unsafe { self.device.map_memory(staging_buffer.memory, 0, self.size, vk::MemoryMapFlags::empty()) }
            .expect("Failed to map memory");
        unsafe { staging_buffer_memory.copy_from_nonoverlapping(data.as_ptr() as *const c_void, data.len()); }
        unsafe { self.device.unmap_memory(staging_buffer.memory); }

        let command_buffer = command_pool.one_time_command_buffer();

        let copy_region = [
            vk::BufferCopy::builder()
                .size(self.size)
                .build()
        ];
        unsafe { self.device.cmd_copy_buffer(*command_buffer, staging_buffer.buffer, self.buffer, &copy_region); }
    }

    fn create_empty_buffer(device: ConstPtr<etna::Device>, physical_device: &etna::PhysicalDevice, size: u64, usage: vk::BufferUsageFlags, memory_properties: vk::MemoryPropertyFlags) -> Buffer {
        let buffer_ci = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE); // expected only to be used by a single queue
        let buffer = unsafe { device.create_buffer(&buffer_ci, None) }
            .expect("Failed to create buffer");

        let memory_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
        let memory_type_index = physical_device.find_memory_type(memory_requirements.memory_type_bits, memory_properties);
        let allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_type_index);

        let memory = unsafe { device.allocate_memory(&allocate_info, None) }
            .expect("Failed to allocate buffer memory");
        unsafe { device.bind_buffer_memory(buffer, memory, 0) }
            .expect("Failed to bind buffer memory");
        Buffer {
            device,
            size,
            buffer,
            memory,
        }
    }
}

pub struct HostMappedBufferCreateInfo {
    pub size: u64,
    pub usage: vk::BufferUsageFlags,
}

pub struct HostMappedBuffer {
    buffer: Buffer,
    mapped_memory: *mut c_void,
}

impl HostMappedBuffer {
    pub fn create(device: ConstPtr<etna::Device>, physical_device: &etna::PhysicalDevice, create_info: HostMappedBufferCreateInfo) -> HostMappedBuffer {
        let buffer = Buffer::create_empty_buffer(
            device,
            physical_device,
            create_info.size,
            create_info.usage,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE
        );
        let mapped_memory = unsafe { buffer.device.map_memory(buffer.memory, 0, create_info.size, vk::MemoryMapFlags::empty()) }
            .expect("Failed to map memory");

        HostMappedBuffer {
            buffer,
            mapped_memory,
        }
    }

    pub fn write_data(&self, data: &[u8]) {
        unsafe { self.mapped_memory.copy_from_nonoverlapping(data.as_ptr() as *const c_void, data.len()); }
    }

    pub fn vk_buffer(&self) -> vk::Buffer {
        self.buffer.buffer
    }
}

impl Drop for HostMappedBuffer {
    fn drop(&mut self) {
        unsafe {
            self.buffer.device.unmap_memory(self.buffer.memory);
        }
    }
}