mod app;
mod audio;
mod graphics;
mod render;

use crate::{
    app::{App, UiCommand},
    graphics::Graphics,
};
use audio::AudioCommand;
use ringbuf::traits::Split;
use ringbuf::HeapRb;
use winit::event_loop::{ControlFlow, EventLoop};

#[cfg(target_arch = "wasm32")]
fn run_app(event_loop: EventLoop<Graphics>, app: App) {
    // Sets up panics to go to the console.error in browser environments
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Error).expect("Couldn't initialize logger");

    // Runs the app async via the browsers event loop
    use winit::platform::web::EventLoopExtWebSys;
    wasm_bindgen_futures::spawn_local(async move {
        event_loop.spawn_app(app);
    });
}

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
    // let event_loop = EventLoop::<()>::new().unwrap();
    let event_loop = EventLoop::<Graphics>::with_user_event().build().unwrap();

    // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
    // dispatched any events. This is ideal for games and similar applications.
    event_loop.set_control_flow(ControlFlow::Poll); // use poll to get 60fs ^^

    // ControlFlow::Wait pauses the event loop if no events are available to process.
    // This is ideal for non-game applications that only update in response to user
    // input, and uses significantly less power/CPU time than ControlFlow::Poll.
    //event_loop.set_control_flow(ControlFlow::Wait);

    let (producer, consumer) = HeapRb::<AudioCommand>::new(64).split();
    let (ui_prod, ui_cons) = HeapRb::<UiCommand>::new(64).split();

    let _stream = audio::init(consumer, ui_prod);
    let app = App::new(producer, ui_cons, &event_loop);
    run_app(event_loop, app);
}
