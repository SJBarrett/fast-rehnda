use egui::Visuals;
use egui::epaint::Shadow;
use winit::event::WindowEvent;
use winit::event_loop::EventLoopWindowTarget;

use crate::etna::{Device, GraphicsSettings, Swapchain};
use crate::rehnda_core::ConstPtr;
use crate::ui::RehndaUi;
use crate::ui::ui_painter::{EguiOutput, UiPainter, ScreenState};

pub struct UiRunner {
    egui_ctx: egui::Context,
    winit_integration: egui_winit::State,
    rehnda_ui: RehndaUi,
    pub egui_renderer: UiPainter,
}


impl UiRunner {
    pub fn create(device: ConstPtr<Device>, event_loop: &EventLoopWindowTarget<()>, graphics_settings: &GraphicsSettings, swapchain: &Swapchain) -> Self {
        let egui_ctx = egui::Context::default();
        let mut dark_visuals = Visuals::dark();
        dark_visuals.window_shadow = Shadow::NONE;
        egui_ctx.set_visuals(dark_visuals);
        UiRunner {
            winit_integration: egui_winit::State::new(event_loop),
            egui_ctx,
            rehnda_ui: RehndaUi::default(),
            egui_renderer: UiPainter::create(device, graphics_settings, swapchain),
        }
    }

    pub fn handle_window_event(&mut self, window_event: &WindowEvent) {
        // TODO handle egui wanting exclusive use of an input event (i.e click on gui not in game)
        let _ = self.winit_integration.on_event(&self.egui_ctx, window_event);
    }

    pub fn update_ui_state(&mut self, window: &winit::window::Window) {
        let new_input = self.winit_integration.take_egui_input(window);
        let full_output = self.egui_ctx.run(new_input, |egui_ctx| self.rehnda_ui.ui(egui_ctx));
        self.winit_integration.handle_platform_output(window, &self.egui_ctx, full_output.platform_output);
        self.egui_renderer.egui_output = EguiOutput {
            clipped_primitives: self.egui_ctx.tessellate(full_output.shapes),
            texture_delta: full_output.textures_delta,
            screen_state: ScreenState {
                size_in_pixels: [window.inner_size().width, window.inner_size().height],
                pixels_per_point: self.egui_ctx.pixels_per_point(),
            },
        };
    }
}
