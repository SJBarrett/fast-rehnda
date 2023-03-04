use egui::DragValue;
use glam::{EulerRot, Mat4, Quat};
use crate::rehnda_core::Vec3;
use crate::scene::{Camera, Scene};

pub struct RehndaUi {
}

impl Default for RehndaUi {
    fn default() -> Self {
        RehndaUi {
        }
    }
}

impl RehndaUi {
    pub fn ui(&mut self, egui_ctx: &egui::Context, scene: &mut Scene)  {
        let mut camera_state = CameraUiState::from_scene(&scene.camera);
        egui::Window::new("Scene").show(egui_ctx, |ui| {
            ui.heading("Camera");
            ui.horizontal(|ui| {
                ui.label("Translation: ");
                ui.add(DragValue::new(&mut camera_state.translation.x).speed(0.03));
                ui.add(DragValue::new(&mut camera_state.translation.y).speed(0.03));
                ui.add(DragValue::new(&mut camera_state.translation.z).speed(0.03));
            });
            ui.horizontal(|ui| {
                ui.label("Rotation: ");
                ui.add(DragValue::new(&mut camera_state.rotation.x).speed(0.1));
                ui.add(DragValue::new(&mut camera_state.rotation.y).speed(0.1));
                ui.add(DragValue::new(&mut camera_state.rotation.z).speed(0.1));
            });
        });
        camera_state.update_scene(&mut scene.camera);
    }
}

struct CameraUiState {
    translation: Vec3,
    rotation: Vec3,
    scale: Vec3,
}

impl CameraUiState {
    fn from_scene(camera: &Camera) -> Self {
        let (scale, rotation, translation) = camera.transform.to_scale_rotation_translation();
        let (mut rotation_x, mut rotation_y, mut rotation_z) = rotation.to_euler(EulerRot::XYZ);
        rotation_x = rotation_x.to_degrees();
        rotation_y = rotation_y.to_degrees();
        rotation_z = rotation_z.to_degrees();
        CameraUiState {
            translation,
            rotation: Vec3::new(rotation_x, rotation_y, rotation_z),
            scale,
        }
    }

    fn update_scene(&self, camera: &mut Camera) {
        camera.transform = Mat4::from_scale_rotation_translation(self.scale, Quat::from_euler(EulerRot::XYZ, self.rotation.x.to_radians(), self.rotation.y.to_radians(), self.rotation.z.to_radians()), self.translation);
    }
}
