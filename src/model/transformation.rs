use bytemuck_derive::{Pod, Zeroable};
use crate::core::*;

#[repr(C)]
#[derive(Zeroable, Pod, Debug, Copy, Clone)]
pub struct TransformationMatrices {
    pub model: Mat4,
    pub view: Mat4,
    pub projection: Mat4,
}