use crate::audio::{init, AudioCommand};
use crate::graphics::{
    context_menu::{ContextMenu, ContextMenuKind},
    mini_window::{MiniWindow, WindowKind, MIXER_ID, PIANO_ROLL_ID, PLAYLIST_ID, SEQUENCER_ID},
    {bring_to_front, create_graphics, ClickResult, DragResult, Graphics, Rc},
};
use crate::project::{AudioBlock, Instrument, PatternData};
use cpal::{traits::StreamTrait, Stream};
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
// std lib
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread;

#[derive(Debug)]
pub struct MouseState {
    pub x: f32,
    pub y: f32,
    pub left_clicked: bool,
    pub right_clicked: bool,
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub shift_pressed: bool,
    pub left_click_held: bool,
}

#[derive(Debug)]
pub struct PianoRollState {
    pub pattern_id: usize,
    pub instrument_id: u32,
}

// commands that the audio engine sends to the window
pub enum UiCommand {
    LoadProjectFile(String),
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
    project_is_dirty: bool,

    stream: Stream,
    pending_project: Option<String>,

    // keyboard state
    ctrl_pressed: bool,

    // mouse state
    mouse_state: MouseState,
    prev_mouse_x: f32,
    prev_mouse_y: f32,
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
            project_is_dirty: false,
            // mouse state
            prev_mouse_x: 0.0,
            prev_mouse_y: 0.0,

            right_click_held: false,
            mouse_state: MouseState {
                x: 0.0,
                y: 0.0,
                left_clicked: false,
                right_clicked: false,
                scroll_x: 0.0,
                scroll_y: 0.0,
                shift_pressed: false,
                left_click_held: false,
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
                        self.project_is_dirty = true;
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
                    UiCommand::LoadProjectFile(path) => {
                        gfx.project_path = path;
                    }
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
                            self.stream = init(audio_cons, ui_prod, Some(path));
                        }
                        self.project_is_dirty = false;
                    }
                }
            }

            // single draw call — returns click result
            let start = std::time::Instant::now();
            let (result, icon) = gfx.draw(&self.mouse_state, self.project_is_dirty);
            gfx.frame_ms = start.elapsed().as_secs_f32() * 1000.0;
            gfx.window.set_cursor(icon);

            // dispatch audio commands based on what was clicked
            match result {
                ClickResult::LoadPianoRoll(piano_state) => {
                    gfx.piano_roll_state = Some(piano_state);

                    if let Some(win) = gfx.mini_windows.iter_mut().find(|w| matches!(w.window_kind, WindowKind::PianoRoll)) {
                        bring_to_front(&mut gfx.z_order, PIANO_ROLL_ID);
                        win.is_open = true;
                    }
                }
                ClickResult::TogglePianoRollWindow => {
                    if let Some(win) = gfx.mini_windows.iter_mut().find(|w| matches!(w.window_kind, WindowKind::PianoRoll)) {
                        if !win.is_open {
                            bring_to_front(&mut gfx.z_order, PIANO_ROLL_ID);
                        }
                        win.is_open = !win.is_open;
                    }
                }
                ClickResult::ToggleNote(pattern_id, instrument_id, step_idx, pitch) => {
                    self.producer
                        .try_push(AudioCommand::ToggleNote(pattern_id, instrument_id, step_idx, pitch))
                        .ok();

                    // also update UI state
                    if let Some(pattern) = gfx.patterns.iter_mut().find(|p| p.id == pattern_id) {
                        if let Some(seq) = pattern.sequences.iter_mut().find(|s| s.instrument_id == instrument_id) {
                            let note = &mut seq.steps[step_idx];
                            if note.velocity > 0.0 && note.pitch == pitch {
                                *note = crate::project::Note::default();
                            } else {
                                *note = crate::project::Note { velocity: 95.0, pitch };
                            }
                        } else {
                            let mut steps = vec![crate::project::Note::default(); 32];
                            steps[step_idx] = crate::project::Note { velocity: 95.0, pitch };
                            pattern.sequences.push(crate::project::Sequence { instrument_id, steps });
                        }
                    }
                    self.project_is_dirty = true;
                }
                ClickResult::CloseContextMenu => {
                    gfx.context_menu = None;
                }
                ClickResult::OpenPatternMenu(x, y, id) => {
                    gfx.context_menu = Some(ContextMenu {
                        kind: ContextMenuKind::PatternContext(id),
                        x,
                        y,
                        height: 128.0,
                        width: 128.0,
                    });
                }
                ClickResult::OpenTrackMenu(x, y, pattern_id, track_id) => {
                    gfx.context_menu = Some(ContextMenu {
                        kind: ContextMenuKind::TrackContext(pattern_id, track_id),
                        x,
                        y,
                        height: 128.0,
                        width: 128.0,
                    });
                }
                ClickResult::ChangeBpmDown => {
                    gfx.bpm -= 1.0;
                    self.producer.try_push(AudioCommand::ChangeBpm(gfx.bpm)).ok();
                    self.project_is_dirty = true;
                }
                ClickResult::ChangeBpmUp => {
                    gfx.bpm += 1.0;
                    self.producer.try_push(AudioCommand::ChangeBpm(gfx.bpm)).ok();
                    self.project_is_dirty = true;
                }
                ClickResult::SelectPattern(id) => {
                    gfx.active_pattern_id = id;
                }
                ClickResult::ToggleTrackWindow(track) => {
                    // update piano roll state to show this track
                    gfx.piano_roll_state = Some(PianoRollState {
                        pattern_id: gfx.active_pattern_id,
                        instrument_id: gfx.instruments[track].data.id,
                    });

                    if let Some(pos) = gfx.mini_windows.iter().position(|w| w.window_kind == WindowKind::InstrumentDetail(track)) {
                        gfx.mini_windows[pos].is_open = !gfx.mini_windows[pos].is_open;
                    } else {
                        gfx.mini_windows.push(MiniWindow {
                            x: 128.0,
                            y: 128.0,
                            width: 400.0,
                            height: 300.0,
                            title: gfx.instruments[track].data.name.clone(),
                            is_open: true,
                            window_kind: WindowKind::InstrumentDetail(track),
                        });
                        let new_id = gfx.mini_windows.len() - 1;
                        gfx.z_order.push(new_id);
                    }
                }
                ClickResult::AddPlaylist => {
                    self.producer.try_push(AudioCommand::AddPattern).ok();
                    self.project_is_dirty = true;
                }
                ClickResult::DeletePlaylistPattern(id) => {
                    self.producer.try_push(AudioCommand::DeleteAudioBlock(id)).ok();
                    self.project_is_dirty = true;
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
                    });
                    self.project_is_dirty = true;
                }
                ClickResult::DeletePattern(id) => {
                    // delete from audio state
                    self.producer.try_push(AudioCommand::DeletePattern(id)).ok();

                    // delete from ui state
                    gfx.patterns.retain(|p| p.id != id);
                    gfx.events.retain(|e| {
                        if let crate::project::AudioBlockType::Pattern(pid) = e.block_type {
                            pid != id
                        } else {
                            true
                        }
                    });
                    for (i, p) in gfx.patterns.iter_mut().enumerate() {
                        p.id = i;
                    }

                    // close menu
                    gfx.context_menu = None;
                    self.project_is_dirty = true;
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
                ClickResult::ToggleStep(pattern_id, instrument_id, step) => {
                    self.producer.try_push(AudioCommand::ToggleStep(pattern_id, instrument_id, step)).ok();
                    self.project_is_dirty = true;
                }
                ClickResult::Stop => {
                    // reset song to 0:00
                    self.producer.try_push(AudioCommand::Stop).ok();
                }
                ClickResult::ToggleTrackMute(track) => {
                    self.producer.try_push(AudioCommand::ToggleTrackMute(track)).ok();
                    self.project_is_dirty = true;
                }
                ClickResult::ChangeBpm(bpm) => {
                    self.producer.try_push(AudioCommand::ChangeBpm(bpm)).ok();
                    self.project_is_dirty = true;
                }
                ClickResult::TogglePlay => {
                    gfx.is_playing = !gfx.is_playing;

                    self.producer.try_push(AudioCommand::TogglePlay).ok();
                }
                ClickResult::DeleteTrack(i) => {
                    self.producer.try_push(AudioCommand::DeleteTrack(i)).ok();
                    self.project_is_dirty = true;
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

                ClickResult::None => {
                    if self.mouse_state.left_clicked {
                        gfx.context_menu = None;
                    }
                }
            }

            // consume the interactions
            self.mouse_state.left_clicked = false;
            self.mouse_state.right_clicked = false;
            self.mouse_state.scroll_x = 0.0;
            self.mouse_state.scroll_y = 0.0;

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

                win_attr = win_attr.with_inner_size(winit::dpi::LogicalSize::new(1800, 1200)).with_title("Glacier");
                let window = Rc::new(event_loop.create_window(win_attr).expect("create window err."));
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
                    let piano_roll_win = &gfx.mini_windows[PIANO_ROLL_ID];
                    if playlist_win.is_open
                        && playlist_win.is_hovered(self.mouse_state.x, self.mouse_state.y)
                        && !piano_roll_win.is_hovered(self.mouse_state.x, self.mouse_state.y)
                    {
                        if self.mouse_state.shift_pressed {
                            if !(gfx.playlist_scroll_x == 0.0 && self.mouse_state.scroll_y < 0.0) {
                                gfx.playlist_scroll_x += self.mouse_state.scroll_y * 35.0;
                            }
                        } else {
                            if !(gfx.playlist_scroll_y == 0.0 && self.mouse_state.scroll_y < 0.0) {
                                gfx.playlist_scroll_y += self.mouse_state.scroll_y * 35.0;
                            }
                        }
                    } else if piano_roll_win.is_open && piano_roll_win.is_hovered(self.mouse_state.x, self.mouse_state.y) {
                        if self.mouse_state.shift_pressed {
                            if !(gfx.piano_roll_scroll_x == 0.0 && self.mouse_state.scroll_y < 0.0) {
                                gfx.piano_roll_scroll_x += self.mouse_state.scroll_y * 35.0;
                            }
                        } else {
                            if !(gfx.piano_roll_scroll_y == 0.0 && self.mouse_state.scroll_y < 0.0) {
                                gfx.piano_roll_scroll_y = (gfx.piano_roll_scroll_y + self.mouse_state.scroll_y * 35.0).clamp(0.0, 1448.0);
                            }
                        }
                    }
                }
            }

            // mouse event
            WindowEvent::MouseInput { state, button, .. } => {
                if state.is_pressed() && button == MouseButton::Left {
                    // change state
                    self.mouse_state.left_click_held = true;
                    self.mouse_state.left_clicked = true;

                    // redraw
                    self.draw(&event_loop);
                } else {
                    // change state
                    self.mouse_state.left_click_held = false;
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
                    if self.mouse_state.left_click_held {
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
