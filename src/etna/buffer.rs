use std::mem::ManuallyDrop;
use std::os::raw::c_void;
use std::ptr::NonNull;

use ash::vk;
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme};
use log::{debug, info};

use crate::rehnda_core::ConstPtr;
use crate::etna;

pub struct Buffer {
    device: ConstPtr<etna::Device>,
    pub size: u64,
    pub buffer: vk::Buffer,
    pub allocation: ManuallyDrop<Allocation>,
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
            self.device.free_allocation(ManuallyDrop::take(&mut self.allocation));
        }
    }
}

pub struct BufferCreateInfo<'a> {
    pub data: &'a[u8],
    pub usage: vk::BufferUsageFlags,
}

impl Buffer {
    pub fn create_buffer_with_data(device: ConstPtr<etna::Device>, create_info: BufferCreateInfo) -> Buffer {
        let empty_buffer = Self::create_empty_buffer(device, create_info.data.len() as u64, create_info.usage, MemoryLocation::CpuToGpu);

        let mapped_memory = empty_buffer.allocation.mapped_ptr().unwrap().as_ptr();
        unsafe { mapped_memory.copy_from_nonoverlapping(create_info.data.as_ptr() as *const c_void, create_info.data.len()); }

        empty_buffer
    }

    pub fn create_and_initialize_buffer_with_staging_buffer(device: ConstPtr<etna::Device>, command_pool: &etna::CommandPool, create_info: BufferCreateInfo) -> Buffer {
        let mut buffer = Self::create_empty_buffer(device, create_info.data.len() as u64, create_info.usage, MemoryLocation::GpuOnly);
        buffer.populate_buffer_using_staging_buffer(command_pool, create_info.data);
        buffer
    }

    fn populate_buffer_using_staging_buffer(&mut self, command_pool: &etna::CommandPool, data: &[u8]) {
        let staging_buffer = Self::create_empty_buffer(
            self.device,
            self.size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryLocation::CpuToGpu,
        );

        let staging_buffer_memory = staging_buffer.allocation.mapped_ptr().unwrap().as_ptr();
        unsafe { staging_buffer_memory.copy_from_nonoverlapping(data.as_ptr() as *const c_void, data.len()); }

        let command_buffer = command_pool.one_time_command_buffer();

        let copy_region = [
            vk::BufferCopy::builder()
                .size(self.size)
                .build()
        ];
        unsafe { self.device.cmd_copy_buffer(*command_buffer, staging_buffer.buffer, self.buffer, &copy_region); }
    }

    fn create_empty_buffer(device: ConstPtr<etna::Device>, size: u64, usage: vk::BufferUsageFlags, memory_location: MemoryLocation) -> Buffer {
        let buffer_ci = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE); // expected only to be used by a single queue
        let buffer = unsafe { device.create_buffer(&buffer_ci, None) }
            .expect("Failed to create buffer");

        let memory_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
        let allocation = device.allocate(&AllocationCreateDesc {
            name: "Empty buffer",
            requirements: memory_requirements,
            location: memory_location,
            linear: true,
            allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        })
            .expect("Failed to allocate memory for empty buffer");

        unsafe { device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset()) }
            .expect("Failed to bind buffer memory");
        Buffer {
            device,
            size,
            buffer,
            allocation: ManuallyDrop::new(allocation),
        }
    }
}

pub struct HostMappedBufferCreateInfo {
    pub size: u64,
    pub usage: vk::BufferUsageFlags,
}

pub struct HostMappedBuffer {
    buffer: Buffer,
    mapped_memory: NonNull<c_void>,
}

impl HostMappedBuffer {
    pub fn create(device: ConstPtr<etna::Device>, create_info: HostMappedBufferCreateInfo) -> HostMappedBuffer {
        let buffer = Buffer::create_empty_buffer(
            device,
            create_info.size,
            create_info.usage,
            MemoryLocation::CpuToGpu,
        );
        let mapped_memory = buffer.allocation.mapped_ptr().unwrap();

        HostMappedBuffer {
            buffer,
            mapped_memory,
        }
    }

    pub fn write_data(&self, data: &[u8]) {
        unsafe { self.mapped_memory.as_ptr().copy_from_nonoverlapping(data.as_ptr() as *const c_void, data.len()); }
    }

    pub fn size(&self) -> u64 {
        self.buffer.size
    }

    pub fn vk_buffer(&self) -> vk::Buffer {
        self.buffer.buffer
    }
}

unsafe impl Send for HostMappedBuffer {}
unsafe impl Sync for HostMappedBuffer {}