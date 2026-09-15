//! Glacier application root. Allocate ringbuffers and initialize audio and graphic engines

pub mod app;
pub mod audio;
pub mod config;
pub mod graphics;
pub mod project;

use crate::{
    app::{App, UiCommand},
    audio::AudioCommand,
    graphics::Graphics,
};
use ringbuf::{traits::Split, HeapRb};
use winit::event_loop::{ControlFlow, EventLoop};

const BUFFER_SIZE: usize = 64;

/// Main method to start application
pub fn main() {
    // Initializes the log builder from the environment
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("error")).init();

    // Load developer mode if needed
    let dev_mode = std::env::args().any(|a| a == "--dev");

    // DEVELOPER MODE: start app with project loaded for testing
    let default_project = if dev_mode {
        Some("assets/projects/dev.toml".to_string())
    } else {
        None
    };

    // Create heap allocated ring buffers for thread communication
    let (audio_producer, audio_consumer) = HeapRb::<AudioCommand>::new(BUFFER_SIZE).split();
    let (ui_producer, ui_consumer) = HeapRb::<UiCommand>::new(BUFFER_SIZE).split();

    // Start building a new event loop, with a Graphics user event
    let event_loop = EventLoop::<Graphics>::with_user_event().build().unwrap();
    // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't dispatched any events.
    event_loop.set_control_flow(ControlFlow::Poll);

    // Load cached application configuration
    let user_config = config::load();

    // Start the audio stream
    let audio_stream = audio::init(audio_consumer, ui_producer, default_project);

    // Combine audio and ui buffers to create app logic owning the audio stream
    let mut app = App::new(
        audio_producer,
        ui_consumer,
        &event_loop,
        audio_stream,
        user_config,
    );

    // Run the application with the event loop on the calling thread.
    if let Err(e) = event_loop.run_app(&mut app) {
        log::error!("event loop exited with error: {e}");
    }
}
