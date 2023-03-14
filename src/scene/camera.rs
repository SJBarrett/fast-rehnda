use bevy_ecs::prelude::*;
use bevy_time::Time;
use bytemuck_derive::{Pod, Zeroable};
use winit::event::{KeyboardInput, VirtualKeyCode};

use crate::rehnda_core::{Mat4, Vec3};

#[repr(C)]
#[derive(Zeroable, Pod, Debug, Copy, Clone)]
pub struct ViewProjectionMatrices {
    pub view: Mat4,
    pub projection: Mat4,
}

#[derive(Resource)]
pub struct Camera {
    pub position: Vec3,
    pub front: Vec3,
    pub up: Vec3,
    pub yaw: f32,
    pub pitch: f32,
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
        let mut projection = Mat4::perspective_rh_gl(fov_y_degrees.to_radians(), aspect_ratio, z_near, z_far);
        projection.y_axis[1] *= -1.0;
        Camera {
            up: (0.0, 1.0, 0.0).into(),
            front: (0.0, 0.0, 1.0).into(),
            position: (0.0, 0.0, -1.0).into(),
            yaw: 0.0,
            pitch: 0.0,
            projection,
            fov_y: fov_y_degrees.to_radians(),
            z_near,
            z_far,
            aspect_ratio,
        }
    }

    pub fn update_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.projection = OPENGL_TO_VULKAN_MATRIX * Mat4::perspective_rh_gl(self.fov_y, aspect_ratio, self.z_near, self.z_far);
        self.projection.y_axis[1] *= -1.0;
    }

    pub fn to_view_proj(&self) -> ViewProjectionMatrices {
        ViewProjectionMatrices {
            view: Mat4::look_at_rh(self.position, self.position + self.front, self.up),
            projection: self.projection,
        }
    }
}

pub fn camera_input_system(time: Res<Time>, mut camera: ResMut<Camera>, mut keyboard_events: EventReader<KeyboardInput>) {
    let movement_speed = time.delta_seconds() * 50.0;
    let rotation_speed = time.delta_seconds() * 200.0;
    let facing_direction = camera.front;
    let up = camera.up;
    for input in keyboard_events.iter() {
        match input.virtual_keycode {
            Some(VirtualKeyCode::W) => camera.position += facing_direction * movement_speed,
            Some(VirtualKeyCode::S) => camera.position -= facing_direction * movement_speed,
            Some(VirtualKeyCode::A) => camera.position -= facing_direction.cross(up).normalize() * movement_speed,
            Some(VirtualKeyCode::D) => camera.position += facing_direction.cross(up) * movement_speed,
            Some(VirtualKeyCode::Q) => {
                camera.yaw -= rotation_speed;
            }
            Some(VirtualKeyCode::E) => {
                camera.yaw += rotation_speed;
            }
            _ => {}
        }
    }

    let x = camera.yaw.to_radians().cos() * camera.pitch.to_radians().cos();
    let y = camera.pitch.to_radians().sin();
    let z = camera.yaw.to_radians().sin() * camera.pitch.to_radians().cos();
    camera.front = Vec3::new(x, y, z).normalize();
}
