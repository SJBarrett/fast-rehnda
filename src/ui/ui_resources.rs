use std::ops::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use egui::epaint::Shadow;
use egui::{Context, Visuals};
use winit::event::WindowEvent;
use winit::event_loop::EventLoopWindowTarget;

use crate::ui::ui_painter::{EguiOutput, UiPainter};

#[derive(Resource)]
pub struct UiContext {
    pub egui_ctx: egui::Context,
    pub state: egui_winit::State,
}

impl UiContext {
    pub fn take_egui_input(&mut self, window: &winit::window::Window) -> egui::RawInput {
        self.state.take_egui_input(window)
    }

    pub fn run(&self, new_input: egui::RawInput, run_ui: impl FnOnce(&Context)) -> egui::FullOutput {
        self.egui_ctx.run(new_input, run_ui)
    }

    pub fn handle_platform_output(
        &mut self,
        window: &winit::window::Window,
        platform_output: egui::PlatformOutput,
    ) {
        self.state.handle_platform_output(window, &self.egui_ctx, platform_output);
    }
}

#[derive(Resource)]
pub struct UiOutput {
    pub run_output: EguiOutput,
}

pub fn init_ui_resources(event_loop: &EventLoopWindowTarget<()>) -> (UiContext, UiOutput) {
    let egui_ctx = egui::Context::default();
    let mut dark_visuals = Visuals::dark();
    dark_visuals.window_shadow = Shadow::NONE;
    egui_ctx.set_visuals(dark_visuals);
    (UiContext { egui_ctx, state: egui_winit::State::new(event_loop) }, UiOutput { run_output: EguiOutput::default() })
}
