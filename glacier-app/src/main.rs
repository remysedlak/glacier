mod app; // event loop
mod audio; // audio engine
mod config;
mod graphics; // graphics engine
mod project; // serialize data

use crate::{
    app::{App, UiCommand},
    audio::AudioCommand,
    config::UserSettings,
    graphics::Graphics,
};
use ringbuf::{traits::Split, HeapRb};
use winit::event_loop::{ControlFlow, EventLoop};

fn run_app(event_loop: EventLoop<Graphics>, mut app: App) {
    // Allows the setting of the log level through RUST_LOG env var.
    // It also allows wgpu logs to be seen.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("error")).init();

    // Runs the app on the current thread.
    let _ = event_loop.run_app(&mut app);
}

fn main() {
    let dev_mode = std::env::args().any(|a| a == "--dev");

    // <T> (T -> AppEvent) extends regular platform specific events (resize, mouse, etc.).
    // This allows our app to inject custom events and handle them alongside regular ones.
    let event_loop = EventLoop::<Graphics>::with_user_event().build().unwrap();

    // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't dispatched any events.
    event_loop.set_control_flow(ControlFlow::Poll);

    // Create heap allocated ring buffers for thread communication, capacity of 64 commands
    let (audio_prod, audio_cons) = HeapRb::<AudioCommand>::new(64).split();
    let (ui_prod, ui_cons) = HeapRb::<UiCommand>::new(64).split();

    // start the audio stream with empty ringbuffers and no project
    let default_project = if dev_mode { Some("assets/projects/dev.toml".to_string()) } else { None };
    let audio_stream = audio::init(audio_cons, ui_prod, default_project);

    let user_settings: UserSettings = config::load();

    // combine audio and ui buffers to create app logic owning the audio stream
    let app = App::new(audio_prod, ui_cons, &event_loop, audio_stream, user_settings);
    run_app(event_loop, app);
}
