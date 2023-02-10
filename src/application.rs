use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::EventLoop;
use crate::etna;

const WINDOW_TITLE: &str = "Fast Rehnda";
const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

pub struct Application {
    window: winit::window::Window,
    entry: ash::Entry,
    instance: etna::Instance,
    device: etna::Device,
    surface: etna::Surface,
    swapchain: etna::Swapchain,
}

// https://github.com/unknownue/vulkan-tutorial-rust/blob/master/src/tutorials/00_base_code.rs
impl Application {
    pub fn new(event_loop: &EventLoop<()>) -> Application {
        let window = winit::window::WindowBuilder::new()
            .with_title(WINDOW_TITLE)
            .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
            .build(event_loop)
            .expect("Failed to create window.");
        let entry = ash::Entry::linked();
        let instance = etna::Instance::new(&entry);
        let surface = etna::Surface::new(&entry, &instance, window.raw_display_handle(), window.raw_window_handle()).expect("Failed to create surface");
        let physical_device = instance.pick_physical_device(&surface);
        let device = etna::Device::create(&instance, &surface, physical_device);
        let swapchain = etna::Swapchain::create(&instance, &device, &surface, &instance.find_queue_families(&surface, physical_device), &surface.query_best_swapchain_creation_details(&window, physical_device));

        Application {
            window,
            entry,
            instance,
            device,
            surface,
            swapchain,
        }
    }

    pub fn main_loop(self, event_loop: EventLoop<()>) {
        event_loop.run(move |event, _, control_flow| {
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    println!("Close requested, stopping");
                    control_flow.set_exit();
                }
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput {
                        input: KeyboardInput {
                            virtual_keycode,
                            state,
                            ..
                        },
                        ..
                    },
                    ..
                } => {
                    if let (Some(VirtualKeyCode::Escape), ElementState::Pressed) = (virtual_keycode, state) {
                        control_flow.set_exit();
                    }
                }
                Event::MainEventsCleared => {
                    // application update code
                }
                _ => (),
            }
        })
    }
}

impl Drop for Application {
    fn drop(&mut self) {}
}

