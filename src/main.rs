extern crate core;

use log::LevelFilter;
use simplelog::{ColorChoice, Config, TerminalMode, TermLogger};
use winit::event_loop::EventLoop;

use crate::application::Application;

mod etna;
mod application;


fn main() {
    TermLogger::init(LevelFilter::Info, Config::default(), TerminalMode::Mixed, ColorChoice::Auto).unwrap();

    let event_loop = EventLoop::new();
    let application = Application::new(&event_loop);

    application.main_loop(event_loop);
}
