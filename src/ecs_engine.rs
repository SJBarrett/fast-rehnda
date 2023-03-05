use std::sync::Arc;
use bevy_app::App;

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ShouldRun;
use log::info;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::event::WindowEvent;
use winit::event_loop::EventLoopWindowTarget;

use crate::etna;
use crate::etna::{CommandPool, Device, draw_system, FrameRenderer, Instance, PhysicalDevice, Surface, Swapchain};
use crate::etna::material_pipeline::DescriptorManager;
use crate::rehnda_core::LongLivedObject;
use crate::scene::{Scene, scene_builder};
use crate::ui::UiRunner;

pub struct EcsEngine {
    // sync objects above here
    app: App,
}

#[derive(Resource)]
struct EtnaContext {
    entry: ash::Entry,
}

#[derive(Resource)]
struct EtnaWindow {
    window: winit::window::Window,
}

impl EtnaWindow {
    fn is_minimized(&self) -> bool {
        self.window.inner_size().height == 0 || self.window.inner_size().width == 0
    }
}

#[derive(SystemLabel)]
pub struct RenderLabel;

impl EcsEngine {
    pub fn new(window: winit::window::Window, event_loop: &EventLoopWindowTarget<()>) -> EcsEngine {
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
        let scene = scene_builder::basic_scene(device.ptr(), physical_device.ptr(), &swapchain, &mut descriptor_manager);

        let frame_renderer = FrameRenderer::create(device.ptr(), &command_pool, &mut descriptor_manager);

        let mut app = App::new();
        let etna_context = EtnaContext {
            entry,
        };
        app.insert_resource(EtnaWindow {
            window
        });
        app.insert_resource(etna_context);
        app.insert_resource(instance);
        app.insert_resource(surface);
        app.insert_resource(physical_device);
        app.insert_resource(device);
        app.insert_resource(command_pool);
        app.insert_resource(swapchain);
        app.insert_resource(descriptor_manager);
        app.insert_resource(scene);
        app.insert_resource(frame_renderer);
        app.add_system_set(SystemSet::new()
            .label(RenderLabel)
            .with_run_criteria(should_render)
            .with_system(draw_system)
        );
        EcsEngine {
            app,
        }
    }


    pub fn render(&mut self) {
        self.app.update();
    }

    pub fn handle_window_event(&mut self, window_event: &WindowEvent) {

    }
}

fn should_render(window: Res<EtnaWindow>) -> ShouldRun {
    if window.is_minimized() {
        ShouldRun::No
    } else {
        ShouldRun::Yes
    }
}

impl Drop for EcsEngine {
    fn drop(&mut self) {
        unsafe { self.app.world.resource::<LongLivedObject<Device>>().device_wait_idle().expect("Failed to wait for the device to be idle") };
        self.app.world.remove_resource::<Scene>();
        self.app.world.remove_resource::<CommandPool>();
        self.app.world.remove_resource::<FrameRenderer>();
        // self.world.remove_resource::<UiRunner>();
        self.app.world.remove_resource::<DescriptorManager>();
        self.app.world.remove_resource::<Swapchain>();
        self.app.world.remove_resource::<Surface>();
        self.app.world.remove_resource::<LongLivedObject<PhysicalDevice>>();
        self.app.world.remove_resource::<LongLivedObject<Device>>();
        self.app.world.remove_resource::<LongLivedObject<Instance>>();
        self.app.world.remove_resource::<EtnaContext>();
    }
}