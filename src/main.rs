mod app;
mod audio;
mod color;
mod graphics;
mod project;
mod ui;
use crate::{
    app::{App, UiCommand},
    audio::AudioCommand,
    graphics::Graphics,
};
use ringbuf::traits::Split;
use ringbuf::HeapRb;
use winit::event_loop::{ControlFlow, EventLoop};

#[cfg(not(target_arch = "wasm32"))]
fn run_app(event_loop: EventLoop<Graphics>, mut app: App) {
    // Allows the setting of the log level through RUST_LOG env var.
    // It also allows wgpu logs to be seen.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("error")).init();

    // Runs the app on the current thread.
    let _ = event_loop.run_app(&mut app);
}

fn main() {
    // <T> (T -> AppEvent) extends regular platform specific events (resize, mouse, etc.).
    // This allows our app to inject custom events and handle them alongside regular ones.
    let event_loop = EventLoop::<Graphics>::with_user_event().build().unwrap();

    // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
    // dispatched any events. This is ideal for games and similar applications.
    event_loop.set_control_flow(ControlFlow::Poll); // use poll to get 60fs ^^

    // Create heap allocated ring buffers, capacity of 64 audio or ui commands for interprocess communication
    let (audio_prod, audio_cons) = HeapRb::<AudioCommand>::new(64).split();
    let (ui_prod, ui_cons) = HeapRb::<UiCommand>::new(64).split();

    // start the audio stream with buffers that product ui commands asnd consume in audio commands
    let stream = audio::init(audio_cons, ui_prod, "projects/two_songs_project.toml".to_string()); // hard coded intro song for dev work

    // start the ui with buffers that produce audio commands and consume in ui commands
    let app = App::new(audio_prod, ui_cons, &event_loop, stream);
    run_app(event_loop, app);
}
