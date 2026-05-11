use crate::audio::{init, AudioCommand};
use crate::graphics::ui::MouseState;
use crate::graphics::{bring_to_front, create_graphics, ui::WindowKind, ClickResult, DragResult, Graphics, Rc, MIXER_ID, PLAYLIST_ID, SEQUENCER_ID};
use crate::project::{AudioBlock, Instrument, PatternData};
use cpal::traits::StreamTrait;
use cpal::Stream;
use rfd::FileDialog;
use ringbuf::{
    traits::{Consumer, Producer, Split},
    {HeapCons, HeapProd, HeapRb},
};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread;
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
    LoadInstrument(Instrument),
    LoadBpm(f32),
    LoadMasterVolume(f32),
    ShutdownComplete,
    SaveComplete,
    LoadPattern(PatternData),
    LoadEvent(AudioBlock),
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

    stream: Stream,
    pending_project: Option<String>,

    // keyboard state
    ctrl_pressed: bool,

    // mouse state
    mouse_state: MouseState,
    prev_mouse_x: f32,
    prev_mouse_y: f32,
    left_click_held: bool,
    right_click_held: bool,

    instrument_file_dialog_rx: Option<Receiver<Option<PathBuf>>>,
    project_file_dialog_rx: Option<Receiver<Option<PathBuf>>>,
}

// app created for the main event loop
impl App {
    pub fn new(producer: HeapProd<AudioCommand>, consumer: HeapCons<UiCommand>, event_loop: &EventLoop<Graphics>, stream: Stream) -> Self {
        Self {
            producer,
            consumer,
            state: State::Init(Some(event_loop.create_proxy())),
            stream,
            pending_project: None,
            ctrl_pressed: false,
            instrument_file_dialog_rx: None,
            project_file_dialog_rx: None,

            // mouse state
            prev_mouse_x: 0.0,
            prev_mouse_y: 0.0,
            left_click_held: false,
            right_click_held: false,
            mouse_state: MouseState {
                x: 0.0,
                y: 0.0,
                left_clicked: false,
                right_clicked: false,
                scroll_x: 0.0,
                scroll_y: 0.0,
                shift_pressed: false,
            },
        }
    }

    // if the state is ready, draw each frame and handle it's click results
    fn draw(&mut self, event_loop: &ActiveEventLoop) {
        let mut should_exit = false;

        if let State::Ready(gfx) = &mut self.state {
            // handle instrument dialog
            if let Some(rx) = &self.instrument_file_dialog_rx {
                match rx.try_recv() {
                    Ok(Some(path)) => {
                        self.producer
                            .try_push(AudioCommand::AddInstrument(path.to_str().unwrap().to_string()))
                            .ok();
                        self.instrument_file_dialog_rx = None;
                    }
                    Ok(None) => {
                        self.instrument_file_dialog_rx = None;
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => {
                        self.instrument_file_dialog_rx = None;
                    }
                }
            }
            // handle project dialog
            if let Some(rx) = &self.project_file_dialog_rx {
                match rx.try_recv() {
                    Ok(Some(path)) => {
                        gfx.is_playing = false;
                        self.pending_project = Some(path.to_str().unwrap().to_string());
                        self.producer.try_push(AudioCommand::SaveProject).ok();
                        self.project_file_dialog_rx = None;
                    }
                    Ok(None) => {
                        self.project_file_dialog_rx = None;
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => {
                        self.project_file_dialog_rx = None;
                    }
                }
            }

            // consume audio -> ui commands
            while let Some(cmd) = self.consumer.try_pop() {
                match cmd {
                    UiCommand::LoadInstrument(instrument) => {
                        gfx.load_instrument(instrument);
                    }
                    UiCommand::LoadBpm(bpm) => {
                        gfx.bpm = bpm;
                    }
                    UiCommand::LoadEvent(event) => {
                        gfx.load_event(event);
                    }
                    UiCommand::LoadPattern(pattern) => {
                        gfx.load_pattern(pattern);
                    }
                    UiCommand::LoadMasterVolume(fl) => {
                        gfx.master_volume = fl;
                    }
                    UiCommand::StepAdvanced(size) => {
                        gfx.active_step = size;
                        gfx.request_redraw();
                    }
                    UiCommand::ShutdownComplete => {
                        let _ = self.stream.pause();
                        should_exit = true;
                    }
                    UiCommand::SaveComplete => {
                        if let Some(path) = self.pending_project.take() {
                            gfx.instruments.clear();
                            gfx.patterns.clear();
                            let _ = self.stream.pause();
                            let (_prod, audio_cons) = HeapRb::<AudioCommand>::new(64).split();
                            let (ui_prod, ui_cons) = HeapRb::<UiCommand>::new(64).split();
                            self.producer = _prod;
                            self.consumer = ui_cons;
                            self.stream = init(audio_cons, ui_prod, path);
                        }
                    }
                }
            }

            // single draw call — returns click result
            let (result, icon) = gfx.draw(&self.mouse_state);
            gfx.window.set_cursor(icon);
            // consume the interactions
            self.mouse_state.left_clicked = false;
            self.mouse_state.right_clicked = false;
            self.mouse_state.scroll_x = 0.0;
            self.mouse_state.scroll_y = 0.0;

            // dispatch audio commands based on what was clicked
            match result {
                ClickResult::AddPlaylist => {
                    self.producer.try_push(AudioCommand::AddPattern).ok();
                }
                ClickResult::DeletePlaylistPattern(id) => {
                    self.producer.try_push(AudioCommand::DeleteAudioBlock(id)).ok();
                }
                ClickResult::AddPlaylistPattern(track, start_step, length, block_type) => {
                    self.producer
                        .try_push(AudioCommand::CreateAudioBlock(track, start_step, length, block_type.clone()))
                        .ok();
                    gfx.events.push(AudioBlock {
                        id: gfx.events.len(),
                        track: track,
                        start_step,
                        length: length as u32,
                        block_type,
                    })
                }
                ClickResult::ToggleSequencerWindow => {
                    if let Some(win) = gfx.mini_windows.iter_mut().find(|w| matches!(w.window_kind, WindowKind::Sequencer)) {
                        if !win.is_open {
                            bring_to_front(&mut gfx.z_order, SEQUENCER_ID);
                        }
                        win.is_open = !win.is_open;
                    }
                }
                ClickResult::ToggleMixerWindow => {
                    if let Some(win) = gfx.mini_windows.iter_mut().find(|w| matches!(w.window_kind, WindowKind::Mixer)) {
                        if !win.is_open {
                            bring_to_front(&mut gfx.z_order, MIXER_ID);
                        }

                        win.is_open = !win.is_open;
                    }
                }
                ClickResult::TogglePlaylistWindow => {
                    if let Some(win) = gfx.mini_windows.iter_mut().find(|w| matches!(w.window_kind, WindowKind::Playlist)) {
                        if !win.is_open {
                            bring_to_front(&mut gfx.z_order, PLAYLIST_ID);
                        }
                        win.is_open = !win.is_open;
                    }
                }
                ClickResult::Step(pattern_id, instrument_id, step) => {
                    self.producer.try_push(AudioCommand::ToggleStep(pattern_id, instrument_id, step)).ok();
                }
                ClickResult::Stop => {
                    // reset song to 0:00
                    self.producer.try_push(AudioCommand::Stop).ok();
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
                    if self.project_file_dialog_rx.is_none() {
                        let (tx, rx) = std::sync::mpsc::channel::<Option<PathBuf>>();
                        self.project_file_dialog_rx = Some(rx);
                        thread::spawn(move || {
                            let file = FileDialog::new().add_filter("toml", &["toml"]).set_directory("/").pick_file();
                            tx.send(file).ok()
                        });
                    }
                }
                ClickResult::InstrumentFileDialog => {
                    // only allow one instrument to be added at a time.
                    if self.instrument_file_dialog_rx.is_none() {
                        // tx -> open file dialog
                        let (tx, rx) = std::sync::mpsc::channel::<Option<PathBuf>>();
                        self.instrument_file_dialog_rx = Some(rx); // store it
                        thread::spawn(move || {
                            let file = FileDialog::new()
                                .add_filter("wav", &["wav"])
                                .add_filter("mp3", &["mp3"])
                                .set_directory("/")
                                .pick_file();
                            tx.send(file).ok();
                        });
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
                    win_attr = win_attr.with_inner_size(winit::dpi::LogicalSize::new(1800, 1200)).with_title("Glacier");
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

            // keyboard event
            WindowEvent::KeyboardInput { event, .. } => {
                if !event.state.is_pressed() {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::ControlLeft) => {
                            self.ctrl_pressed = false;
                        }
                        PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight) => {
                            self.mouse_state.shift_pressed = false;
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
                        PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight) => {
                            self.mouse_state.shift_pressed = true;
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

            // scroll wheel on mouse
            WindowEvent::MouseWheel { delta, .. } => {
                // delta
                match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        // x = horizontal, y = vertical
                        // y is negative when scrolling down
                        self.mouse_state.scroll_x = x;
                        self.mouse_state.scroll_y = y;
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        // pos.x, pos.y — from trackpads
                        self.mouse_state.scroll_x = pos.x as f32;
                        self.mouse_state.scroll_y = pos.y as f32;
                    }
                }

                // handle playlist scrolling
                if let State::Ready(gfx) = &mut self.state {
                    let playlist_win = &gfx.mini_windows[PLAYLIST_ID];
                    if playlist_win.is_open && playlist_win.is_hovered(self.mouse_state.x, self.mouse_state.y) {
                        if self.mouse_state.shift_pressed {
                            if !(gfx.playlist_scroll_x == 0.0 && self.mouse_state.scroll_y < 0.0) {
                                gfx.playlist_scroll_x += self.mouse_state.scroll_y * 35.0;
                            }
                        } else {
                            if !(gfx.playlist_scroll_y == 0.0 && self.mouse_state.scroll_y < 0.0) {
                                gfx.playlist_scroll_y += self.mouse_state.scroll_y * 35.0;
                            }
                        }
                    }
                }
            }

            // mouse event
            WindowEvent::MouseInput { state, button, .. } => {
                if state.is_pressed() && button == MouseButton::Left {
                    // change state
                    self.left_click_held = true;
                    self.mouse_state.left_clicked = true;

                    // redraw
                    self.draw(&event_loop);
                } else {
                    // change state
                    self.left_click_held = false;
                    self.mouse_state.left_clicked = false;
                    if let State::Ready(gfx) = &mut self.state {
                        gfx.dragging = false;
                        gfx.dragging_window = None; // is this here?
                        gfx.dragging_knob = None;
                    }
                }
                if state.is_pressed() && button == MouseButton::Right {
                    // change state
                    self.mouse_state.right_clicked = true;
                    self.right_click_held = true;
                    // redraw
                    self.draw(&event_loop);
                } else {
                    // change state
                    self.right_click_held = false;
                    self.mouse_state.right_clicked = false;
                }
            }

            // moving mouse on the mouse pad
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_state.x = position.x as f32;
                self.mouse_state.y = position.y as f32;
                let delta_y = position.y as f32 - self.prev_mouse_y;
                let delta_x = position.x as f32 - self.prev_mouse_x;
                self.prev_mouse_y = position.y as f32;
                self.prev_mouse_x = position.x as f32;

                if let State::Ready(gfx) = &mut self.state {
                    if self.left_click_held {
                        match gfx.handle_drag(position.x as f32, position.y as f32, delta_y, delta_x) {
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
