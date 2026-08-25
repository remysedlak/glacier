//! main method. Build thread ring buffers, start the audio stream, and run the app's event loop.

mod app;
mod audio;
mod config;
mod graphics;
mod project;

use crate::{
    app::{App, UiCommand},
    audio::AudioCommand,
    graphics::Graphics,
};
use ringbuf::{traits::Split, HeapRb};
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    // Initializes the log builder from the environment
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("error")).init();

    // load developer mode if needed
    let dev_mode = std::env::args().any(|a| a == "--dev");

    // DEVELOPER MODE: start app with project loaded for testing
    let default_project = if dev_mode {
        Some("assets/projects/dev.toml".to_string())
    } else {
        None
    };

    // Create heap allocated ring buffers for thread communication, capacity of 64 items
    let (audio_producer, audio_consumer) = HeapRb::<AudioCommand>::new(64).split();
    let (ui_producer, ui_consumer) = HeapRb::<UiCommand>::new(64).split();

    // start the audio stream
    let audio_stream = audio::init(audio_consumer, ui_producer, default_project);

    // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't dispatched any events.
    let event_loop = EventLoop::<Graphics>::with_user_event().build().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    // load user's local native app configuration
    let user_config = config::load();

    // combine audio and ui buffers to create app logic owning the audio stream
    let mut app = App::new(
        audio_producer,
        ui_consumer,
        &event_loop,
        audio_stream,
        user_config,
    );

    // Run the application with the event loop on the calling thread.
    let _ = event_loop.run_app(&mut app);
}
