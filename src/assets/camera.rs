use bevy_ecs::prelude::*;
use bevy_time::Time;
use bytemuck_derive::{Pod, Zeroable};
use winit::event::{KeyboardInput, VirtualKeyCode};

use crate::rehnda_core::{Mat4, Vec3};
use crate::rehnda_core::input::{InputState, KeyState};

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

enum CameraMovementType {
    Orbit,
    Fps,
}

pub struct CameraMovementState {
    movement_type: CameraMovementType,
    orbit_rotation: f32,
    orbit_elevation: f32,
    orbit_target_distance: f32,
}

impl Default for CameraMovementState {
    fn default() -> Self {
        Self {
            movement_type: CameraMovementType::Orbit,
            orbit_rotation: 0.0,
            orbit_elevation: 0.0,
            orbit_target_distance: 15.0,
        }
    }
}

pub fn camera_input_system(time: Res<Time>, mut camera_movement_state: Local<CameraMovementState>, mut camera: ResMut<Camera>, input_state: Res<InputState>) {
    if input_state.is_just_down(VirtualKeyCode::T) {
        match camera_movement_state.movement_type {
            CameraMovementType::Orbit => {
                camera_movement_state.movement_type = CameraMovementType::Fps;
            }
            CameraMovementType::Fps => {
                camera_movement_state.movement_type = CameraMovementType::Orbit;
            }
        }
    }
    match camera_movement_state.movement_type {
        CameraMovementType::Orbit => {
            handle_orbit_movement(&time, &mut camera, &mut camera_movement_state, &input_state);
        }
        CameraMovementType::Fps => {
            handle_fps_movement(&time, &mut camera, &input_state);
        }
    }
}

fn handle_orbit_movement(time: &Time, camera: &mut Camera, camera_movement_state: &mut CameraMovementState, input_state: &InputState) {
    let rotate_speed = time.delta_seconds() * 100.0;
    let zoom_speed = time.delta_seconds() * 10.0;
    if input_state.is_down(VirtualKeyCode::W) {
        camera_movement_state.orbit_elevation += rotate_speed;
    }
    if input_state.is_down(VirtualKeyCode::S) {
        camera_movement_state.orbit_elevation -= rotate_speed;
    }
    if input_state.is_down(VirtualKeyCode::A) {
        camera_movement_state.orbit_rotation -= rotate_speed;
    }
    if input_state.is_down(VirtualKeyCode::D) {
        camera_movement_state.orbit_rotation += rotate_speed;
    }
    if input_state.is_down(VirtualKeyCode::Q) {
        camera_movement_state.orbit_target_distance += zoom_speed;
    }
    if input_state.is_down(VirtualKeyCode::E) {
        camera_movement_state.orbit_target_distance -= zoom_speed;
    }
    camera_movement_state.orbit_target_distance = camera_movement_state.orbit_target_distance.clamp(0.5, 100.0);

    let target_distance = camera_movement_state.orbit_target_distance;
    let x = target_distance * camera_movement_state.orbit_rotation.to_radians().sin() * camera_movement_state.orbit_elevation.to_radians().cos();
    let y = target_distance * camera_movement_state.orbit_elevation.to_radians().sin();
    let z = target_distance * camera_movement_state.orbit_rotation.to_radians().cos() * camera_movement_state.orbit_elevation.to_radians().cos();
    camera.position = (x, y, z).into();
    camera.front = (-camera.position).normalize();
}

fn handle_fps_movement(time: &Time, camera: &mut Camera, input_state: &InputState) {
    let mut speed_modifier = time.delta_seconds();
    if input_state.is_down(VirtualKeyCode::LShift) {
        speed_modifier *= 0.1;
    }
    let movement_speed = speed_modifier * 20.0;
    let rotation_speed = speed_modifier * 80.0;
    let facing_direction = camera.front;
    let up = camera.up;
    if input_state.is_down(VirtualKeyCode::W) {
        camera.position += facing_direction * movement_speed;
    }
    if input_state.is_down(VirtualKeyCode::S) {
        camera.position -= facing_direction * movement_speed;
    }
    if input_state.is_down(VirtualKeyCode::A) {
        camera.position -= facing_direction.cross(up).normalize() * movement_speed;
    }
    if input_state.is_down(VirtualKeyCode::D) {
        camera.position += facing_direction.cross(up) * movement_speed;
    }
    if input_state.is_down(VirtualKeyCode::Space) {
        camera.position += up * movement_speed;
    }
    if input_state.is_down(VirtualKeyCode::LControl) {
        camera.position -= up * movement_speed;
    }
    if input_state.is_down(VirtualKeyCode::Q) {
        camera.yaw -= rotation_speed;
    }
    if input_state.is_down(VirtualKeyCode::E) {
        camera.yaw += rotation_speed;
    }


    let x = camera.yaw.to_radians().cos() * camera.pitch.to_radians().cos();
    let y = camera.pitch.to_radians().sin();
    let z = camera.yaw.to_radians().sin() * camera.pitch.to_radians().cos();
    camera.front = Vec3::new(x, y, z).normalize();
}