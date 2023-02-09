use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::EventLoop;
use crate::rvk;

const WINDOW_TITLE: &str = "Fast Rehnda";
const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

pub struct Application {
    entry: ash::Entry,
    instance: rvk::Instance,
    device: rvk::Device,
}

// https://github.com/unknownue/vulkan-tutorial-rust/blob/master/src/tutorials/00_base_code.rs
impl Application {
    pub fn new() -> Application {
        let entry = ash::Entry::linked();
        let instance =  rvk::Instance::new(&entry);
        let device = rvk::Device::create(&instance, instance.pick_physical_device());

        Application {
            entry,
            instance,
            device,
        }
    }

    pub fn init_window(event_loop: &EventLoop<()>) -> winit::window::Window {
        winit::window::WindowBuilder::new()
            .with_title(WINDOW_TITLE)
            .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
            .build(event_loop)
            .expect("Failed to create window.")

    }

    pub fn main_loop(mut self, event_loop: EventLoop<()>, window: winit::window::Window) {
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
    fn drop(&mut self) {

    }
}

