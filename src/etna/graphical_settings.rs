use ash::vk;

#[derive(Debug, Copy, Clone)]
pub struct GraphicsSettings {
    // sample more than 1 will enable multisampling
    pub msaa_samples: MsaaSamples,
    // sample rate shading makes shaders be multi-sampled, not just geometry, but at a performance cost
    pub sample_rate_shading_enabled: bool,
}

impl GraphicsSettings {
    pub fn is_msaa_enabled(&self) -> bool {
        !matches!(&self.msaa_samples, MsaaSamples::X1)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum MsaaSamples {
    X1,
    X2,
    X4,
    X8,
    X16,
    X32,
    X64,
}

impl MsaaSamples {
    pub fn to_sample_count_flags(&self) -> vk::SampleCountFlags {
        match self {
            Self::X1 => vk::SampleCountFlags::TYPE_1,
            Self::X2 => vk::SampleCountFlags::TYPE_2,
            Self::X4 => vk::SampleCountFlags::TYPE_4,
            Self::X8 => vk::SampleCountFlags::TYPE_8,
            Self::X16 => vk::SampleCountFlags::TYPE_16,
            Self::X32 => vk::SampleCountFlags::TYPE_32,
            Self::X64 => vk::SampleCountFlags::TYPE_64,
        }
    }
}