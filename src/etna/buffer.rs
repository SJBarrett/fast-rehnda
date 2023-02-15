use std::os::raw::c_void;
use std::sync::Arc;
use ash::vk;
use crate::etna;

pub struct Buffer {
    device: Arc<etna::Device>,
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
    pub size: u64,
    pub usage: vk::BufferUsageFlags,
    pub data: &'a[u8],
}

impl Buffer {
    pub fn create(device: Arc<etna::Device>, physical_device: &etna::PhysicalDevice, create_info: BufferCreateInfo) -> Buffer {
        let buffer_ci = vk::BufferCreateInfo::builder()
            .size(create_info.size)
            .usage(create_info.usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE); // expected only to be used by a single queue
        let buffer = unsafe { device.create_buffer(&buffer_ci, None) }
            .expect("Failed to create buffer");

        let memory_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
        let memory_type_index = physical_device.find_memory_type(memory_requirements.memory_type_bits, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT);
        let allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_type_index);

        let memory = unsafe { device.allocate_memory(&allocate_info, None) }
            .expect("Failed to allocate buffer memory");
        unsafe { device.bind_buffer_memory(buffer, memory, 0) }
            .expect("Failed to bind buffer memory");

        let mapped_memory = unsafe { device.map_memory(memory, 0, create_info.size, vk::MemoryMapFlags::empty()) }
            .expect("Failed to map memory");
        unsafe { mapped_memory.copy_from_nonoverlapping(create_info.data.as_ptr() as *const c_void, create_info.data.len()); }
        unsafe { device.unmap_memory(memory); }


        Buffer {
            device,
            buffer,
            memory,
        }
    }
}