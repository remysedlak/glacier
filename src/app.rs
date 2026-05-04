use crate::audio;
use crate::audio::AudioCommand;
use crate::graphics::{create_graphics, ClickResult, DragResult, Graphics, Rc};
use cpal::traits::StreamTrait;
use cpal::Stream;
use rfd::FileDialog;
use ringbuf::{
    traits::{Consumer, Producer, Split},
    {HeapCons, HeapProd, HeapRb},
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

// commands that the audio engine sends to the window
pub enum UiCommand {
    StepAdvanced(usize),
    LoadTrack(usize, String, Vec<f32>, bool, f32),
    LoadBpm(f32),
    LoadMasterVolume(f32),
    ShutdownComplete,
    SaveComplete,
}

// the app is in initializing state or its ready to draw
enum State {
    Ready(Graphics),
    Init(Option<EventLoopProxy<Graphics>>),
}

// gui app state
pub struct App {
    producer: HeapProd<AudioCommand>,
    consumer: HeapCons<UiCommand>,
    state: State,
    mouse_x: f32,
    mouse_y: f32,
    prev_mouse_y: f32,
    stream: Stream,
    pending_project: Option<String>,
    ctrl_pressed: bool,
    clicked: bool,
    left_click_held: bool,
}

impl App {
    pub fn new(producer: HeapProd<AudioCommand>, consumer: HeapCons<UiCommand>, event_loop: &EventLoop<Graphics>, stream: Stream) -> Self {
        Self {
            producer,
            consumer,
            state: State::Init(Some(event_loop.create_proxy())),
            mouse_x: 0.0,
            mouse_y: 0.0,
            prev_mouse_y: 0.0,
            stream,
            pending_project: None,
            ctrl_pressed: false,
            left_click_held: false,
            clicked: false,
        }
    }

    // if the state is ready, draw the frame and handle click results
    fn draw(&mut self, event_loop: &ActiveEventLoop) {
        let mut should_exit = false;

        if let State::Ready(gfx) = &mut self.state {
            // consume audio -> ui commands
            while let Some(cmd) = self.consumer.try_pop() {
                match cmd {
                    UiCommand::StepAdvanced(size) => {
                        gfx.active_step = size;
                        gfx.request_redraw();
                    }
                    UiCommand::LoadTrack(i, name, steps, mute, vol) => {
                        gfx.load_track(i, name, steps, mute, vol);
                    }
                    UiCommand::LoadBpm(bpm) => {
                        gfx.bpm = bpm;
                    }
                    UiCommand::LoadMasterVolume(fl) => {
                        gfx.master_volume = fl;
                    }
                    UiCommand::ShutdownComplete => {
                        let _ = self.stream.pause();
                        should_exit = true;
                    }
                    UiCommand::SaveComplete => {
                        if let Some(path) = self.pending_project.take() {
                            let _ = self.stream.pause();
                            let (audio_prod, audio_cons) = HeapRb::<AudioCommand>::new(64).split();
                            let (ui_prod, ui_cons) = HeapRb::<UiCommand>::new(64).split();
                            self.producer = audio_prod;
                            self.consumer = ui_cons;
                            self.stream = audio::init(audio_cons, ui_prod, path);
                        }
                    }
                }
            }

            // single draw call — returns click result
            let result = gfx.draw(self.mouse_x, self.mouse_y, self.clicked);
            self.clicked = false; // consume the click

            // dispatch audio commands based on what was clicked
            match result {
                ClickResult::Step(track, step) => {
                    self.producer.try_push(AudioCommand::ToggleStep(track, step)).ok();
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
                ClickResult::DeleteTrack(i) => {
                    self.producer.try_push(AudioCommand::DeleteTrack(i)).ok();
                }
                ClickResult::ProjectFileDialog => {
                    let file = FileDialog::new().add_filter("toml", &["toml"]).set_directory("/").pick_file();
                    match file {
                        Some(x) => {
                            gfx.is_playing = false;
                            self.pending_project = Some(x.to_str().unwrap().to_string());
                            self.producer.try_push(AudioCommand::SaveProject).ok();
                        }
                        None => println!("no file chosen..."),
                    }
                }
                ClickResult::InstrumentFileDialog => {
                    let file = FileDialog::new()
                        .add_filter("wav", &["wav"])
                        .add_filter("mp3", &["mp3"])
                        .set_directory("/")
                        .pick_file();
                    match file {
                        Some(x) => {
                            self.producer
                                .try_push(AudioCommand::AddInstrument(x.to_str().unwrap().to_string()))
                                .ok();
                        }
                        None => println!("no file chosen..."),
                    }
                }
                ClickResult::None => {}
            }

            gfx.request_redraw();
        }

        if should_exit {
            self.state = State::Init(None);
            event_loop.exit();
        }
    }

    fn resized(&mut self, size: PhysicalSize<u32>) {
        if let State::Ready(gfx) = &mut self.state {
            gfx.resize(size);
        }
    }
}

impl ApplicationHandler<Graphics> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let State::Init(proxy) = &mut self.state {
            if let Some(proxy) = proxy.take() {
                let mut win_attr = Window::default_attributes();

                #[cfg(not(target_arch = "wasm32"))]
                {
                    win_attr = win_attr
                        .with_inner_size(winit::dpi::LogicalSize::new(1400, 800))
                        .with_title("Glacier");
                }

                #[cfg(target_arch = "wasm32")]
                {
                    use winit::platform::web::WindowAttributesExtWebSys;
                    win_attr = win_attr.with_append(true);
                    win_attr = win_attr.with_inner_size(winit::dpi::LogicalSize::new(1400, 800));
                }

                let window = Rc::new(event_loop.create_window(win_attr).expect("create window err."));

                #[cfg(target_arch = "wasm32")]
                wasm_bindgen_futures::spawn_local(create_graphics(window, proxy));

                #[cfg(not(target_arch = "wasm32"))]
                pollster::block_on(create_graphics(window, proxy));
            }
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, graphics: Graphics) {
        graphics.request_redraw();
        self.state = State::Ready(graphics);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.producer.try_push(AudioCommand::ShutDown).ok();
                if let State::Ready(gfx) = &mut self.state {
                    gfx.request_redraw();
                }
            }
            WindowEvent::Resized(size) => self.resized(size),
            WindowEvent::RedrawRequested => self.draw(&event_loop),

            WindowEvent::KeyboardInput { event, .. } => {
                if !event.state.is_pressed() {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::ControlLeft) => {
                            self.ctrl_pressed = false;
                        }
                        _ => {}
                    }
                }

                if event.state.is_pressed() {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Space) => {
                            self.producer.try_push(AudioCommand::TogglePlay).ok();
                            if let State::Ready(gfx) = &mut self.state {
                                gfx.is_playing = !gfx.is_playing;
                            }
                        }
                        PhysicalKey::Code(KeyCode::ControlLeft) => {
                            self.ctrl_pressed = true;
                        }
                        PhysicalKey::Code(KeyCode::KeyS) => {
                            if self.ctrl_pressed {
                                self.producer.try_push(AudioCommand::SaveProject).ok();
                            }
                        }
                        _ => {}
                    }
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if state.is_pressed() && button == MouseButton::Left {
                    self.left_click_held = true;
                    self.clicked = true;
                    self.draw(&event_loop); // single draw, result handled inside
                } else {
                    self.left_click_held = false;
                    self.clicked = false;
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_x = position.x as f32;
                self.mouse_y = position.y as f32;
                let delta_y = position.y as f32 - self.prev_mouse_y;
                self.prev_mouse_y = position.y as f32;

                if let State::Ready(gfx) = &mut self.state {
                    if self.left_click_held {
                        match gfx.handle_drag(position.x as f32, position.y as f32, delta_y) {
                            DragResult::None => {}
                            DragResult::DragVolumeSlider(fl) => {
                                self.producer.try_push(AudioCommand::ChangeMasterVolume(fl)).ok();
                                gfx.request_redraw();
                            }
                            DragResult::DragVolumeKnob(i, fl) => {
                                self.producer.try_push(AudioCommand::ChangeTrackVolume(i, fl)).ok();
                                gfx.request_redraw();
                            }
                        }
                    } else {
                        gfx.dragging_knob = None;
                    }
                }
            }
            _ => {}
        }
    }
}
