use bevy_ecs::prelude::{NonSend, Res, ResMut};
use bevy_ecs::system::NonSendMut;
use egui::DragValue;
use glam::{EulerRot, Mat4, Quat};

use crate::ecs_engine::EtnaWindow;
use crate::rehnda_core::Vec3;
use crate::scene::Camera;
use crate::ui::ui_painter::{EguiOutput, ScreenState};

pub fn ui_builder_system(mut camera: ResMut<Camera>, egui_ctx: NonSend<egui::Context>, mut winit_state: NonSendMut<egui_winit::State>, mut ui_output: ResMut<EguiOutput>, window: Res<EtnaWindow>) {
    let new_input = winit_state.take_egui_input(&window.winit_window);
    let full_output = egui_ctx.run(new_input, |egui_ctx| {
        draw_ui(egui_ctx, &mut camera);
    });

    winit_state.handle_platform_output(&window.winit_window,  &egui_ctx, full_output.platform_output);
    ui_output.screen_state = ScreenState {
        size_in_pixels: [window.winit_window.inner_size().width, window.winit_window.inner_size().height],
        pixels_per_point: egui_ctx.pixels_per_point(),
    };
    ui_output.clipped_primitives = egui_ctx.tessellate(full_output.shapes);
    ui_output.texture_delta = full_output.textures_delta;
}

fn draw_ui(egui_ctx: &egui::Context, camera: &mut Camera) {
    let mut camera_state = CameraUiState::from_scene(camera);
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
    camera_state.update_scene(camera);
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
