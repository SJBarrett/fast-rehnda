use bevy_ecs::prelude::{NonSend, Res, ResMut};
use bevy_ecs::system::{NonSendMut, Query};
use egui::{DragValue, Separator, Ui};
use glam::{EulerRot, Mat4, Quat};

use crate::ecs_engine::EtnaWindow;
use crate::assets::Camera;
use crate::assets::demo_scenes::Actor;
use crate::assets::render_object::RenderObject;
use crate::ui::ui_painter::{EguiOutput, ScreenState};

pub fn ui_builder_system(mut camera: ResMut<Camera>, mut render_objects: Query<(&RenderObject, &mut Actor)>, egui_ctx: NonSend<egui::Context>, mut winit_state: NonSendMut<egui_winit::State>, mut ui_output: ResMut<EguiOutput>, window: Res<EtnaWindow>) {
    let new_input = winit_state.take_egui_input(&window.winit_window);
    let full_output = egui_ctx.run(new_input, |egui_ctx| {
        draw_ui(egui_ctx, &mut camera, render_objects);
    });

    winit_state.handle_platform_output(&window.winit_window,  &egui_ctx, full_output.platform_output);
    ui_output.screen_state = ScreenState {
        size_in_pixels: [window.winit_window.inner_size().width, window.winit_window.inner_size().height],
        pixels_per_point: egui_ctx.pixels_per_point(),
    };
    ui_output.clipped_primitives = egui_ctx.tessellate(full_output.shapes);
    ui_output.texture_delta = full_output.textures_delta;
}

fn draw_ui(egui_ctx: &egui::Context, camera: &mut Camera, mut render_objects: Query<(&RenderObject, &mut Actor)>) {
    egui::Window::new("Scene").show(egui_ctx, |ui| {
        ui.heading("Camera");
        ui.label(format!("x: {:.1}, y: {:.1}, z: {:.1}", camera.position.x, camera.position.y, camera.position.z));
        ui.label(format!("yaw: {:.0}, pitch: {:.0}", camera.yaw, camera.pitch));

        ui.heading("Objects");
        for (object, mut actor) in &mut render_objects {
            ui.add(Separator::default());
            ui.label(&actor.name);
            draw_transform(ui, &mut actor);
        }
    });
}

fn draw_transform(ui: &mut Ui, object: &mut Actor) {
    let (scale, rotation, mut translation) = object.transform.to_scale_rotation_translation();
    ui.horizontal(|ui| {
        ui.label("Translation: ");
        ui.add(DragValue::new(&mut translation.x).speed(0.03));
        ui.add(DragValue::new(&mut translation.y).speed(0.03));
        ui.add(DragValue::new(&mut translation.z).speed(0.03));
    });
    object.transform = Mat4::from_scale_rotation_translation(scale, rotation, translation);
}
