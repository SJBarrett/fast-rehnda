use bevy_app::App;
use bevy_ecs::prelude::*;
use bevy_time::TimePlugin;
use egui::epaint::Shadow;
use egui::Visuals;
use log::info;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::event::{KeyboardInput, WindowEvent};
use winit::event_loop::EventLoopWindowTarget;
use winit::window::Window;

use crate::etna::{CommandPool, Device, draw_system, FrameRenderContext, Instance, PhysicalDevice, Surface, Swapchain, swapchain_systems};
use crate::etna::material_pipeline::DescriptorManager;
use crate::rehnda_core::input::{input_systems, InputState};
use crate::rehnda_core::LongLivedObject;
use crate::assets::{AssetManager, camera_input_system, light_source, material_server};
use crate::assets::demo_scenes;
use crate::assets::light_source::LightingDataManager;
use crate::assets::material_server::MaterialServer;
use crate::ui::{EguiOutput, ui_builder_system, UiPainter};

pub struct EcsEngine {
    // sync objects above here
    app: App,
}

#[derive(Resource)]
struct EtnaContext {
    entry: ash::Entry,
}

#[derive(Resource)]
pub struct EtnaWindow {
    pub winit_window: winit::window::Window,
}

impl EtnaWindow {
    fn is_minimized(&self) -> bool {
        self.winit_window.inner_size().height == 0 || self.winit_window.inner_size().width == 0
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum RehndaSet {
    PreUpdate,
    Update,
    Render,
}

impl EcsEngine {
    pub fn new(window: Window, event_loop: &EventLoopWindowTarget<()>) -> EcsEngine {
        let mut app = App::new();
        app.add_plugin(TimePlugin::default());
        Self::initialise_rendering_resources(&mut app, window, event_loop);
        app.init_resource::<InputState>();
        app.init_resource::<MaterialServer>();
        app.add_event::<winit::event::KeyboardInput>();
        app.add_startup_system(material_server::material_startup_system);
        app.add_startup_system(demo_scenes::spheres_scene);
        app.add_systems((
            input_systems::input_system.in_set(RehndaSet::PreUpdate),
        ));
        app.add_system(material_server::material_server_system.in_set(RehndaSet::Render));
        app.add_systems((
            camera_input_system.in_set(RehndaSet::Update),
            light_source::update_lights_system.in_set(RehndaSet::Update),
            ui_builder_system.run_if(should_render).in_set(RehndaSet::Render),
        ));
        app.add_systems((
            draw_system.after(ui_builder_system).run_if(should_render).in_set(RehndaSet::Render),
            swapchain_systems::swap_chain_recreation_system.run_if(swapchain_systems::swap_chain_needs_recreation).after(draw_system).in_set(RehndaSet::Render),
        ));
        app.configure_set(
            RehndaSet::PreUpdate.before(RehndaSet::Update)
        );
        app.configure_set(
            RehndaSet::Update.before(RehndaSet::Render)
        );
        EcsEngine {
            app,
        }
    }

    fn initialise_rendering_resources(app: &mut App, window: Window, event_loop: &EventLoopWindowTarget<()>) {
        let entry = ash::Entry::linked();
        let instance = LongLivedObject::new(Instance::new(&entry));
        let surface = Surface::new(&entry, &instance, window.raw_display_handle(), window.raw_window_handle()).expect("Failed to create surface");
        let physical_device = LongLivedObject::new(PhysicalDevice::pick_physical_device(instance.ptr(), &surface));
        info!("Graphics Settings: {:?}", physical_device.graphics_settings);
        let device = LongLivedObject::new(Device::create(&instance, &surface, &physical_device));
        let command_pool = CommandPool::create(device.ptr(), physical_device.queue_families().graphics_family);
        let swapchain = Swapchain::create(
            &instance,
            device.ptr(),
            &physical_device,
            &surface,
            &command_pool,
            &physical_device.queue_families(),
            surface.query_best_swapchain_creation_details(window.inner_size(), physical_device.handle()),
        );
        let mut descriptor_manager = DescriptorManager::create(device.ptr());
        let asset_manager = AssetManager::create(device.ptr(), physical_device.ptr(), CommandPool::create(device.ptr(), physical_device.queue_families().graphics_family));
        let frame_renderer = FrameRenderContext::create(device.ptr(), &command_pool, &mut descriptor_manager);

        // ui resources
        let egui_ctx = egui::Context::default();
        let mut dark_visuals = Visuals::dark();
        dark_visuals.window_shadow = Shadow::NONE;
        egui_ctx.set_visuals(dark_visuals);
        app.insert_non_send_resource(egui::Context::default());
        app.insert_non_send_resource(egui_winit::State::new(event_loop));
        app.insert_resource(EguiOutput::default());
        app.insert_resource(UiPainter::create(device.ptr(), &physical_device.graphics_settings, &swapchain));
        app.insert_resource(LightingDataManager::new(device.ptr(), &mut descriptor_manager));
        let etna_context = EtnaContext {
            entry,
        };
        app.insert_resource(EtnaWindow {
            winit_window: window
        });
        app.insert_resource(etna_context);
        app.insert_resource(instance);
        app.insert_resource(surface);
        app.insert_resource(physical_device);
        app.insert_resource(device);
        app.insert_resource(command_pool);
        app.insert_resource(swapchain);
        app.insert_resource(descriptor_manager);
        app.insert_resource(frame_renderer);
        app.insert_resource(asset_manager);
    }

    pub fn render(&mut self) {
        self.app.update();
    }

    pub fn handle_window_event(&mut self, window_event: &WindowEvent) {
        let world = self.app.world.cell();
        let winit_state = &mut world.non_send_resource_mut::<egui_winit::State>();
        if let WindowEvent::KeyboardInput { input, .. } = window_event {
            world.send_event(*input);
        }

        let _ = winit_state.on_event(&world.non_send_resource::<egui::Context>(), window_event);
    }
}

fn should_render(window: Res<EtnaWindow>) -> bool {
    !window.is_minimized()
}

impl Drop for EcsEngine {
    fn drop(&mut self) {
        unsafe { self.app.world.resource::<LongLivedObject<Device>>().device_wait_idle().expect("Failed to wait for the device to be idle") };
        self.app.world.remove_resource::<EguiOutput>();
        self.app.world.remove_resource::<UiPainter>();
        self.app.world.remove_resource::<LightingDataManager>();
        self.app.world.remove_resource::<MaterialServer>();
        self.app.world.remove_resource::<AssetManager>();
        self.app.world.remove_resource::<CommandPool>();
        self.app.world.remove_resource::<FrameRenderContext>();
        self.app.world.remove_resource::<DescriptorManager>();
        self.app.world.remove_resource::<Swapchain>();
        self.app.world.remove_resource::<Surface>();
        self.app.world.remove_resource::<LongLivedObject<PhysicalDevice>>();
        self.app.world.remove_resource::<LongLivedObject<Device>>();
        self.app.world.remove_resource::<LongLivedObject<Instance>>();
        self.app.world.remove_resource::<EtnaContext>();
    }
}