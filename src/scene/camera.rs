use bevy_ecs::prelude::Resource;
use bytemuck_derive::{Pod, Zeroable};
use crate::rehnda_core::Mat4;

#[repr(C)]
#[derive(Zeroable, Pod, Debug, Copy, Clone)]
pub struct ViewProjectionMatrices {
    pub view: Mat4,
    pub projection: Mat4,
}

#[derive(Resource)]
pub struct Camera {
    pub transform: Mat4,
    pub projection: Mat4,
    aspect_ratio: f32,
    fov_y: f32,
    z_near: f32,
    z_far: f32,
}

const OPENGL_TO_VULKAN_MATRIX: Mat4 = Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
]);

impl Camera {
    pub fn new(fov_y_degrees: f32, aspect_ratio: f32, z_near: f32, z_far: f32) -> Camera {
        let mut projection = OPENGL_TO_VULKAN_MATRIX * Mat4::perspective_rh(fov_y_degrees.to_radians(), aspect_ratio, z_near, z_far);
        projection.y_axis[1] *= -1.0;
        Camera {
            transform: Mat4::IDENTITY,
            projection,
            fov_y: fov_y_degrees.to_radians(),
            z_near,
            z_far,
            aspect_ratio,
        }
    }

    pub fn update_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.projection = Mat4::perspective_rh(self.fov_y, aspect_ratio, self.z_near, self.z_far);
        self.projection.y_axis[1] *= -1.0;
    }

    pub fn to_view_proj(&self) -> ViewProjectionMatrices {
        ViewProjectionMatrices {
            view: self.transform,
            projection: self.projection,
        }
    }
}