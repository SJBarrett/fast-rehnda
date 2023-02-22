use std::sync::Arc;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::EventLoop;

use crate::etna_engine::EtnaEngine;

const WINDOW_TITLE: &str = "Fast Rehnda";
const WINDOW_WIDTH: u32 = 1600;
const WINDOW_HEIGHT: u32 = 1200;

pub struct Application {
    _window: Arc<winit::window::Window>,
    etna_engine: EtnaEngine,
}

// https://github.com/unknownue/vulkan-tutorial-rust/blob/master/src/tutorials/00_base_code.rs
impl Application {
    pub fn new(event_loop: &EventLoop<()>) -> Application {
        let window = Arc::new(winit::window::WindowBuilder::new()
            .with_title(WINDOW_TITLE)
            .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
            .build(event_loop)
            .expect("Failed to create window."));
        let engine = EtnaEngine::new(window.clone());

        Application {
            _window: window,
            etna_engine: engine,
        }
    }

    pub fn main_loop(mut self, event_loop: EventLoop<()>) {
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
                    self.etna_engine.render();
                }
                _ => (),
            }
        });
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        self.etna_engine.wait_idle();
    }
}

