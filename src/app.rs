use crate::audio;
use crate::audio::AudioCommand;
use crate::graphics::{create_graphics, ClickResult, Graphics, Rc};
use cpal::traits::StreamTrait;
use cpal::Stream;
use rfd::FileDialog;
use ringbuf::traits::Split;
use ringbuf::traits::{Consumer, Producer};
use ringbuf::HeapRb;
use ringbuf::{HeapCons, HeapProd};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

// commands that the audio engine sends to the window
pub enum UiCommand {
    StepAdvanced(usize),
    LoadTrack(usize, String, [bool; 16], bool),
    LoadBpm(f32),
    ShutdownComplete,
    SaveComplete,
    // InstrumentAdded(...) when we get there
}

// the app is in initializing state or its ready to draw
enum State {
    Ready(Graphics),
    Init(Option<EventLoopProxy<Graphics>>),
}

pub struct App {
    producer: HeapProd<AudioCommand>,
    consumer: HeapCons<UiCommand>,
    state: State,
    mouse_x: f64,
    mouse_y: f64,
    stream: Stream,
    pending_project: Option<String>,
}

impl App {
    // initalize the event loop on creation
    pub fn new(
        producer: HeapProd<AudioCommand>,
        consumer: HeapCons<UiCommand>,
        event_loop: &EventLoop<Graphics>,
        stream: Stream,
    ) -> Self {
        Self {
            producer,
            consumer,
            state: State::Init(Some(event_loop.create_proxy())),
            mouse_x: 0.0,
            mouse_y: 0.0,
            stream: stream,
            pending_project: None,
        }
    }

    // if the state is ready, draw the frame
    fn draw(&mut self, event_loop: &ActiveEventLoop) {
        if let State::Ready(gfx) = &mut self.state {
            while let Some(cmd) = self.consumer.try_pop() {
                match cmd {
                    UiCommand::StepAdvanced(size) => {
                        gfx.active_step = size;
                    }
                    UiCommand::LoadTrack(i, name, steps, mute) => {
                        gfx.load_track(i, name, steps, mute);
                    }
                    UiCommand::LoadBpm(bpm) => {
                        gfx.bpm = bpm;
                    }
                    UiCommand::ShutdownComplete => {
                        let _ = self.stream.pause();
                        event_loop.exit();
                    }
                    UiCommand::SaveComplete => {
                        if let Some(path) = self.pending_project.take() {
                            // swap out ring buffers
                            let _ = self.stream.pause();
                            let (audio_prod, audio_cons) = HeapRb::<AudioCommand>::new(64).split();
                            let (ui_prod, ui_cons) = HeapRb::<UiCommand>::new(64).split();
                            self.producer = audio_prod;
                            self.consumer = ui_cons;
                            // restore the stream
                            self.stream = audio::init(audio_cons, ui_prod, path);
                        } else {
                        }
                    }
                }
            }
            gfx.draw(self.mouse_x, self.mouse_y);
            gfx.request_redraw(); // add this
        }
    }

    // handles window resizing and min/maximizing
    fn resized(&mut self, size: PhysicalSize<u32>) {
        if let State::Ready(gfx) = &mut self.state {
            gfx.resize(size);
        }
    }
}

// app startup
impl ApplicationHandler<Graphics> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let State::Init(proxy) = &mut self.state {
            if let Some(proxy) = proxy.take() {
                let mut win_attr = Window::default_attributes();

                #[cfg(not(target_arch = "wasm32"))]
                {
                    win_attr = win_attr.with_title("WebGPU example");
                }

                #[cfg(target_arch = "wasm32")]
                {
                    use winit::platform::web::WindowAttributesExtWebSys;
                    win_attr = win_attr.with_append(true);
                }

                let window = Rc::new(
                    event_loop
                        .create_window(win_attr)
                        .expect("create window err."),
                );

                #[cfg(target_arch = "wasm32")]
                wasm_bindgen_futures::spawn_local(create_graphics(window, proxy));

                #[cfg(not(target_arch = "wasm32"))]
                pollster::block_on(create_graphics(window, proxy));
            }
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, graphics: Graphics) {
        // Request a redraw now that graphics are ready
        graphics.request_redraw();
        self.state = State::Ready(graphics);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.producer.try_push(AudioCommand::ShutDown).ok();
                if let State::Ready(gfx) = &mut self.state {
                    gfx.request_redraw();
                }
            }
            WindowEvent::Resized(size) => self.resized(size),
            WindowEvent::RedrawRequested => self.draw(&event_loop),
            WindowEvent::MouseInput { state, button, .. } => {
                if state.is_pressed() && button == MouseButton::Left {
                    if let State::Ready(gfx) = &mut self.state {
                        match gfx.handle_button_click(self.mouse_x, self.mouse_y) {
                            ClickResult::Step(track, step) => {
                                self.producer
                                    .try_push(AudioCommand::ToggleStep(track, step))
                                    .ok();
                            }
                            ClickResult::Mute(track) => {
                                self.producer.try_push(AudioCommand::ToggleMute(track)).ok();
                            }
                            ClickResult::ChangeBpm(bpm) => {
                                self.producer.try_push(AudioCommand::ChangeBpm(bpm)).ok();
                            }
                            ClickResult::TogglePlay => {
                                self.producer.try_push(AudioCommand::TogglePlay).ok();
                            }
                            ClickResult::FileDialog => {
                                let file = FileDialog::new()
                                    .add_filter("toml", &["toml"])
                                    .set_directory("/")
                                    .pick_file();

                                match file {
                                    Some(x) => {
                                        // save and pause the stream
                                        gfx.is_playing = false;
                                        self.pending_project =
                                            Some(x.to_str().unwrap().to_string());
                                        self.producer.try_push(AudioCommand::SaveProject).ok();
                                    }
                                    None => {
                                        println!("no file chosen...")
                                    }
                                }
                            }
                            ClickResult::None => {}
                        }
                    } // closes if let
                    self.draw(&event_loop);
                } // closes if state.is_pressed()
            } // closes MouseInput arm

            WindowEvent::CursorMoved { position, .. } => {
                // position.x and position.y are available here
                self.mouse_x = position.x;
                self.mouse_y = position.y;
                self.draw(&event_loop);
            }
            _ => {}
        }
    }
}
