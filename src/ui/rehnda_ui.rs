use bevy_ecs::prelude::{NonSend, Res, ResMut};
use bevy_ecs::system::{NonSendMut, Query};
use egui::{DragValue, Separator, Ui};

use crate::ecs_engine::EtnaWindow;
use crate::assets::Camera;
use crate::assets::demo_scenes::Actor;
use crate::assets::light_source::PointLight;
use crate::assets::render_object::{RenderObject, Transform};
use crate::rehnda_core::Mat4;
use crate::ui::ui_painter::{EguiOutput, ScreenState};

pub fn ui_builder_system(mut camera: ResMut<Camera>, mut actors: Query<(&Actor, &mut Transform)>, mut lights: Query<&mut PointLight>, egui_ctx: NonSend<egui::Context>, mut winit_state: NonSendMut<egui_winit::State>, mut ui_output: ResMut<EguiOutput>, window: Res<EtnaWindow>) {
    let new_input = winit_state.take_egui_input(&window.winit_window);
    let full_output = egui_ctx.run(new_input, |egui_ctx| {
        draw_ui(egui_ctx, &mut camera, actors, lights);
    });

    winit_state.handle_platform_output(&window.winit_window,  &egui_ctx, full_output.platform_output);
    ui_output.screen_state = ScreenState {
        size_in_pixels: [window.winit_window.inner_size().width, window.winit_window.inner_size().height],
        pixels_per_point: egui_ctx.pixels_per_point(),
    };
    ui_output.clipped_primitives = egui_ctx.tessellate(full_output.shapes);
    ui_output.texture_delta = full_output.textures_delta;
}

fn draw_ui(egui_ctx: &egui::Context, camera: &mut Camera, mut actors: Query<(&Actor, &mut Transform)>, mut lights: Query<(&mut PointLight)>) {
    egui::Window::new("Scene").show(egui_ctx, |ui| {
        ui.heading("Camera");
        ui.label(format!("x: {:.1}, y: {:.1}, z: {:.1}", camera.position.x, camera.position.y, camera.position.z));
        ui.label(format!("yaw: {:.0}, pitch: {:.0}", camera.yaw, camera.pitch));

        ui.heading("Objects");
        for (actor, mut transform) in &mut actors {
            ui.add(Separator::default());
            ui.label(&actor.name);
            draw_transform(ui, &mut transform);
        }

        ui.heading("Lights");
        for (mut light) in &mut lights {
            draw_light(ui, &mut light);
        }
    });
}

fn draw_transform(ui: &mut Ui, transform: &mut Transform) {
    ui.horizontal(|ui| {
        ui.label("Translation: ");
        ui.add(DragValue::new(&mut transform.translation.x).speed(0.03));
        ui.add(DragValue::new(&mut transform.translation.y).speed(0.03));
        ui.add(DragValue::new(&mut transform.translation.z).speed(0.03));
    });
}

fn draw_light(ui: &mut Ui, light: &mut PointLight) {
    let mut color = light.light_color;
    let mut emissivity = light.emissivity;
    ui.horizontal(|ui| {
        ui.label("Color: ");
        ui.add(DragValue::new(&mut color.x).speed(0.03));
        ui.add(DragValue::new(&mut color.y).speed(0.03));
        ui.add(DragValue::new(&mut color.z).speed(0.03));
        ui.label("Emissivity: ");
        ui.add(DragValue::new(&mut emissivity).speed(1));
    });
    light.emissivity = emissivity;
    light.light_color = color;
}