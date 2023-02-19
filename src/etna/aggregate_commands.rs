pub mod image_transitions {
    use ash::vk;
    use crate::etna;

    pub struct TransitionProps {
        pub old_layout: vk::ImageLayout,
        pub src_access_mask: vk::AccessFlags2,
        pub src_stage_mask: vk::PipelineStageFlags2,
        pub new_layout: vk::ImageLayout,
        pub dst_access_mask: vk::AccessFlags2,
        pub dst_stage_mask: vk::PipelineStageFlags2,
    }

    impl TransitionProps {
        pub const fn undefined_to_transfer_dst() -> TransitionProps {
            TransitionProps {
                old_layout: vk::ImageLayout::UNDEFINED,
                src_access_mask: vk::AccessFlags2::empty(),
                src_stage_mask: vk::PipelineStageFlags2::TOP_OF_PIPE,
                new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                dst_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                dst_stage_mask: vk::PipelineStageFlags2::TRANSFER,
            }
        }

        pub const fn transfer_dst_to_shader_read() -> TransitionProps {
            TransitionProps {
                old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                src_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                src_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                dst_access_mask: vk::AccessFlags2::SHADER_READ,
                dst_stage_mask: vk::PipelineStageFlags2::FRAGMENT_SHADER,
            }
        }
    }

    pub fn transition_image_layout(device: &etna::Device, command_buffer: &vk::CommandBuffer, image: vk::Image, transition: &TransitionProps) {
        let image_memory_barrier = vk::ImageMemoryBarrier2::builder()
            .src_access_mask(transition.src_access_mask)
            .src_stage_mask(transition.src_stage_mask)
            .old_layout(transition.old_layout)
            .new_layout(transition.new_layout)
            .dst_stage_mask(transition.dst_stage_mask)
            .dst_access_mask(transition.dst_access_mask)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1)
                .build()
            );
        let dep_info = vk::DependencyInfo::builder()
            .image_memory_barriers(std::slice::from_ref(&image_memory_barrier));
        // make the transition to present happen
        unsafe { device.cmd_pipeline_barrier2(*command_buffer, &dep_info) };
    }
}

