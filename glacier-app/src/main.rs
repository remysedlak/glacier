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

    // <T> (T -> AppEvent) extends regular platform specific events (resize, mouse, etc.).
    // This allows our app to inject custom events and handle them alongside regular ones.

    // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't dispatched any events.
    let event_loop = EventLoop::<Graphics>::with_user_event().build().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    // Create heap allocated ring buffers for thread communication, capacity of 64 commands
    let (audio_prod, audio_cons) = HeapRb::<AudioCommand>::new(64).split();
    let (ui_prod, ui_cons) = HeapRb::<UiCommand>::new(64).split();

    // start the audio stream with default project
    let default_project = if dev_mode {
        Some("assets/projects/dev.toml".to_string())
    } else {
        None
    };
    let audio_stream = audio::init(audio_cons, ui_prod, default_project);

    // combine audio and ui buffers to create app logic owning the audio stream
    let mut app = App::new(audio_prod, ui_cons, &event_loop, audio_stream, config::load());
    let _ = event_loop.run_app(&mut app);
}
