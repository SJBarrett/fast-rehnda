use log::LevelFilter;
use simplelog::{ColorChoice, Config, TerminalMode, TermLogger};
use winit::event_loop::EventLoop;

use crate::application::Application;

mod core;
mod etna;
mod model;
mod application;
mod etna_engine;


fn main() {
    TermLogger::init(LevelFilter::Debug, Config::default(), TerminalMode::Mixed, ColorChoice::Auto).unwrap();

    let event_loop = EventLoop::new();
    let application = Application::new(&event_loop);

    application.main_loop(event_loop);
}
