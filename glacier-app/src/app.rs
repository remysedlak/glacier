// app.rs - main state logic for audio and ui decoupling

use crate::audio::{init, AudioCommand};
use crate::config::{self, UserSettings};
use crate::graphics::mini_window::track;
use crate::graphics::{
    context_menu::{ContextMenu, ContextMenuKind},
    drag::DragResult,
    mini_window::{
        piano_roll::PIANO_ROLL_DEFAULT_Y, sequencer::TRACK_GAP, MiniWindow, WindowKind, MIXER_ID,
        PIANO_ROLL_ID, PLAYLIST_ID, SEQUENCER_ID,
    },
    {bring_to_front, create_graphics, ClickResult, Graphics, Rc},
};
use crate::project::{AudioBlock, PatternData, Track, TrackData};
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
use std::time::Instant;

#[derive(Debug)]
pub struct MouseState {
    // posiiton
    pub x: f32,
    pub y: f32,
    // clicking
    pub left_clicked: bool,
    pub left_double_clicked: bool,
    pub left_click_held: bool,
    pub right_clicked: bool,
    // scrolling
    pub scroll_x: f32,
    pub scroll_y: f32,
    // hovering
    pub hover_state: Option<Instant>,
}

#[derive(Clone, Copy)]
pub struct ScrollOffset {
    pub x: f32,
    pub y: f32,
}
impl Default for ScrollOffset {
    fn default() -> Self {
        ScrollOffset { x: 0.0, y: 0.0 }
    }
}

pub struct PianoRollState {
    pub pattern_id: usize,
    pub track_id: u32,
    pub scroll_offset: ScrollOffset,
}

// commands that the audio engine sends to the window
pub enum UiCommand {
    LoadProjectPath(String),
    StepAdvanced(usize),
    TrackLevel(u32, f32, f32, f32), // track_id, rms_l, rms_r, peak
    MasterLevel(f32, f32, f32),     // rms_l, rms_r, peak
    LoadTrack(Track),
    LoadBpm(f32),
    LoadMasterVolume(f32),
    ShutdownComplete,
    SaveComplete,
    LoadPattern(PatternData),
    LoadEvent(AudioBlock),
    PlayheadPosition(f32),
}

// the app is in initializing state or its ready to draw
enum State {
    Ready(Box<Graphics>),
    Init(Option<EventLoopProxy<Graphics>>),
}

// gui app state
pub struct App {
    // ringbuffer
    producer: HeapProd<AudioCommand>,
    consumer: HeapCons<UiCommand>,

    // app state
    state: State,
    config: UserSettings,

    // audio state
    stream: Stream,
    pending_project: Option<String>,
    project_is_dirty: bool,

    // keyboard state
    ctrl_pressed: bool,
    pub shift_pressed: bool,

    // mouse state
    mouse_state: MouseState,
    prev_mouse_x: f32,
    prev_mouse_y: f32,
    right_click_held: bool,
    last_click_time: Option<std::time::Instant>,

    // file dialog
    track_file_dialog_rx: Option<Receiver<Option<PathBuf>>>,
    project_file_dialog_rx: Option<Receiver<Option<PathBuf>>>,
    track_load_rx: Option<Receiver<(TrackData, Vec<f32>)>>,
    project_save_dialog_rx: Option<Receiver<Option<PathBuf>>>,
}

// app created for the main event loop
impl App {
    pub fn new(
        producer: HeapProd<AudioCommand>,
        consumer: HeapCons<UiCommand>,
        event_loop: &EventLoop<Graphics>,
        stream: Stream,
        config: UserSettings,
    ) -> Self {
        Self {
            producer,
            consumer,
            state: State::Init(Some(event_loop.create_proxy())),
            stream,
            pending_project: None,
            project_is_dirty: false,
            ctrl_pressed: false,
            shift_pressed: false,
            track_file_dialog_rx: None,
            project_file_dialog_rx: None,
            project_save_dialog_rx: None,
            track_load_rx: None,
            prev_mouse_x: 0.0,
            prev_mouse_y: 0.0,
            last_click_time: None,
            config,

            right_click_held: false,
            mouse_state: MouseState {
                x: 0.0,
                y: 0.0,
                left_clicked: false,
                left_double_clicked: false,
                right_clicked: false,
                scroll_x: 0.0,
                scroll_y: 0.0,
                left_click_held: false,
                hover_state: None,
            },
        }
    }

    // if the state is ready, draw each frame and handle it's click results
    fn draw(&mut self, event_loop: &ActiveEventLoop) {
        let mut should_exit = false;

        if let State::Ready(gfx) = &mut self.state {
            // handle track dialog
            if let Some(rx) = &self.track_file_dialog_rx {
                match rx.try_recv() {
                    Ok(Some(path)) => {
                        let path_str = path.to_str().unwrap().to_string();
                        let (tx, load_rx) = std::sync::mpsc::channel();
                        self.track_load_rx = Some(load_rx);
                        std::thread::spawn(move || {
                            let samples = crate::project::path_to_vector(&path_str);
                            let name = std::path::Path::new(&path_str)
                                .file_name()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .to_string();
                            let data = crate::project::TrackData {
                                id: 0,
                                path: path_str,
                                name,
                                is_muted: false,
                                target_volume: 1.0,
                                track_volume: 1.0,
                                root_note: 60,
                            };
                            tx.send((data, samples)).ok();
                        });
                        self.project_is_dirty = true;
                        self.track_file_dialog_rx = None;
                    }
                    Ok(None) => {
                        self.track_file_dialog_rx = None;
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => {
                        self.track_file_dialog_rx = None;
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

            if let Some(rx) = &self.project_save_dialog_rx {
                match rx.try_recv() {
                    Ok(Some(path)) => {
                        let path_str = path.to_str().unwrap().to_string();
                        gfx.project_path = path_str.clone();
                        self.producer
                            .try_push(AudioCommand::SetProjectPath(path_str))
                            .ok();
                        self.producer.try_push(AudioCommand::SaveProject).ok();
                        self.project_save_dialog_rx = None;
                    }
                    Ok(None) => {
                        self.project_save_dialog_rx = None;
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => {
                        self.project_save_dialog_rx = None;
                    }
                }
            }

            // consume audio -> ui commands
            while let Some(cmd) = self.consumer.try_pop() {
                match cmd {
                    UiCommand::PlayheadPosition(beat) => {
                        gfx.playhead_beat = beat;
                    }
                    UiCommand::LoadProjectPath(path) => {
                        gfx.project_path = path;
                    }
                    UiCommand::TrackLevel(track_id, rms_l, rms_r, peak) => {
                        if let Some(track) = gfx.tracks.iter_mut().find(|t| t.data.id == track_id) {
                            track.rms_l = rms_l;
                            track.rms_r = rms_r;
                            track.peak_hold = peak;
                        }
                    }
                    UiCommand::MasterLevel(rms_l, rms_r, peak) => {
                        gfx.master_rms_l = rms_l;
                        gfx.master_rms_r = rms_r;
                        gfx.master_peak = peak;
                    }
                    UiCommand::LoadTrack(track) => {
                        gfx.load_track(track);

                        //ui
                        let win = &mut gfx.mini_windows[SEQUENCER_ID];
                        win.height = 100.0 + TRACK_GAP * gfx.tracks.len() as f32;
                        win.is_open = true;
                        bring_to_front(&mut gfx.z_order, SEQUENCER_ID);
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
                    UiCommand::LoadMasterVolume(volume) => {
                        gfx.master_volume = volume;
                    }
                    UiCommand::StepAdvanced(step) => {
                        gfx.active_step = step;
                        gfx.request_redraw();
                    }
                    UiCommand::ShutdownComplete => {
                        config::save(&self.config);
                        let _ = self.stream.pause();
                        should_exit = true;
                    }
                    UiCommand::SaveComplete => {
                        if let Some(path) = self.pending_project.take() {
                            // reset ui
                            gfx.tracks.clear();
                            gfx.patterns.clear();

                            // reset IPC state
                            let _ = self.stream.pause();
                            let (audio_prod, audio_cons) = HeapRb::<AudioCommand>::new(64).split();
                            let (ui_prod, ui_cons) = HeapRb::<UiCommand>::new(64).split();
                            self.producer = audio_prod;
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
            gfx.window.set_cursor(icon);

            gfx.frame_ms = start.elapsed().as_secs_f32() * 1000.0;

            // start hover timer when tooltip is present, reset when it's not
            if gfx.tooltip.is_some() {
                if self.mouse_state.hover_state.is_none() {
                    self.mouse_state.hover_state = Some(Instant::now());
                }
            } else {
                self.mouse_state.hover_state = None;
            }

            if let Some(rx) = &self.track_load_rx {
                match rx.try_recv() {
                    Ok((data, samples)) => {
                        self.producer
                            .try_push(AudioCommand::LoadTrack(data, samples))
                            .ok();
                        self.track_load_rx = None;
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => {
                        self.track_load_rx = None;
                    }
                }
            }
            // dispatch audio commands based on what was clicked
            match result {
                ClickResult::TogglePatternTray => {
                    gfx.show_pattern_tray = !gfx.show_pattern_tray;
                }
                ClickResult::ToggleTrackTray => {
                    gfx.show_track_tray = !gfx.show_track_tray;
                }
                ClickResult::OpenTrackFileLocation(path) => {
                    // open the track file in the default file system
                    showfile::show_path_in_file_manager(path);
                }
                ClickResult::StartResizeEvent(id) => {
                    gfx.resizing_event = Some(id);
                }
                ClickResult::DuplicatePattern(pattern_id) => {
                    self.producer
                        .try_push(AudioCommand::DuplicatePattern(pattern_id))
                        .ok();
                    self.project_is_dirty = true;
                    gfx.patterns.push(PatternData {
                        id: gfx.patterns.len(),
                        name: format!("Pattern {}", gfx.patterns.len()),
                        sequences: gfx
                            .patterns
                            .iter()
                            .find(|p| p.id == pattern_id)
                            .unwrap()
                            .sequences
                            .clone(),
                    });
                }
                ClickResult::LoadPianoRoll(piano_state) => {
                    gfx.context_menu = None;
                    gfx.piano_roll_state = Some(piano_state);
                    if let Some(win) = gfx
                        .mini_windows
                        .iter_mut()
                        .find(|w| matches!(w.window_kind, WindowKind::PianoRoll))
                    {
                        bring_to_front(&mut gfx.z_order, PIANO_ROLL_ID);
                        win.is_open = true;
                    }
                }
                ClickResult::TogglePianoRollWindow => {
                    if let Some(win) = gfx
                        .mini_windows
                        .iter_mut()
                        .find(|w| matches!(w.window_kind, WindowKind::PianoRoll))
                    {
                        if !win.is_open {
                            bring_to_front(&mut gfx.z_order, PIANO_ROLL_ID);
                        }
                        win.is_open = !win.is_open;
                        gfx.context_menu = None;
                    }
                }
                ClickResult::ToggleNote(pattern_id, track_id, step_idx, pitch) => {
                    self.producer
                        .try_push(AudioCommand::ToggleNote(
                            pattern_id, track_id, step_idx, pitch,
                        ))
                        .ok();

                    // also update UI state
                    if let Some(pattern) = gfx.patterns.iter_mut().find(|p| p.id == pattern_id) {
                        if let Some(seq) = pattern
                            .sequences
                            .iter_mut()
                            .find(|s| s.track_id == track_id)
                        {
                            let note = &mut seq.steps[step_idx];
                            if note.velocity > 0.0 && note.pitch == pitch {
                                *note = crate::project::Note::default();
                            } else {
                                *note = crate::project::Note {
                                    velocity: 95.0,
                                    pitch,
                                };
                            }
                        } else {
                            let mut steps = vec![crate::project::Note::default(); 32];
                            steps[step_idx] = crate::project::Note {
                                velocity: 95.0,
                                pitch,
                            };
                            pattern
                                .sequences
                                .push(crate::project::Sequence { track_id, steps });
                        }
                    }
                    self.project_is_dirty = true;
                }
                ClickResult::CloseContextMenu => {
                    gfx.context_menu = None;
                }
                ClickResult::OpenPatternMenu(x, y, pattern_id) => {
                    gfx.context_menu = Some(ContextMenu {
                        kind: ContextMenuKind::PatternContext(pattern_id),
                        x,
                        y,
                        width: 128.0,
                    });
                }
                ClickResult::OpenTrackMenu(x, y, pattern_id, track_id) => {
                    gfx.context_menu = Some(ContextMenu {
                        kind: ContextMenuKind::TrackContext(pattern_id, track_id),
                        x,
                        y,
                        width: 128.0,
                    });
                }
                ClickResult::ChangeBpmDown => {
                    gfx.bpm -= 1.0;
                    self.producer
                        .try_push(AudioCommand::ChangeBpm(gfx.bpm))
                        .ok();
                    self.project_is_dirty = true;
                }
                ClickResult::ChangeBpmUp => {
                    gfx.bpm += 1.0;
                    self.producer
                        .try_push(AudioCommand::ChangeBpm(gfx.bpm))
                        .ok();
                    self.project_is_dirty = true;
                }
                ClickResult::SelectPattern(pattern_id) => {
                    gfx.active_pattern_id = pattern_id;
                }
                ClickResult::ToggleTrackWindow(track) => {
                    // update piano roll state to show this track
                    gfx.piano_roll_state = Some(PianoRollState {
                        pattern_id: gfx.active_pattern_id,
                        track_id: gfx.tracks[track].data.id,
                        scroll_offset: ScrollOffset {
                            x: 0.0,
                            y: PIANO_ROLL_DEFAULT_Y,
                        },
                    });

                    if let Some(pos) = gfx
                        .mini_windows
                        .iter()
                        .position(|w| w.window_kind == WindowKind::TrackDetail(track))
                    {
                        gfx.mini_windows[pos].is_open = !gfx.mini_windows[pos].is_open;
                    } else {
                        gfx.mini_windows.push(MiniWindow {
                            x: 128.0,
                            y: 128.0,
                            width: 600.0,
                            height: 500.0,
                            title: gfx.tracks[track].data.name.clone(),
                            is_open: true,
                            window_kind: WindowKind::TrackDetail(track),
                        });
                        let new_id = gfx.mini_windows.len() - 1;
                        gfx.z_order.push(new_id);
                    }
                }
                ClickResult::CreatePattern => {
                    self.producer.try_push(AudioCommand::AddPattern).ok();
                    self.project_is_dirty = true;
                }
                ClickResult::DeletePlaylistPattern(id) => {
                    // ui
                    gfx.events.retain(|e| e.id != id);
                    // audio
                    self.producer
                        .try_push(AudioCommand::DeleteAudioBlock(id))
                        .ok();
                    self.project_is_dirty = true;
                }
                ClickResult::AddPlaylistPattern(track, start_step, length, block_type) => {
                    self.producer
                        .try_push(AudioCommand::CreateAudioBlock(
                            track,
                            start_step,
                            length,
                            block_type.clone(),
                        ))
                        .ok();
                    gfx.events.push(AudioBlock {
                        id: gfx.events.len(),
                        track,
                        start_step,
                        length: length as u32,
                        block_type,
                    });
                    self.project_is_dirty = true;
                }
                ClickResult::DeletePattern(pattern_id) => {
                    // delete from audio state
                    self.producer
                        .try_push(AudioCommand::DeletePattern(pattern_id))
                        .ok();

                    // delete from ui state
                    gfx.patterns.retain(|p| p.id != pattern_id);
                    gfx.events.retain(|e| {
                        if let crate::project::AudioBlockType::Pattern(pid) = e.block_type {
                            pid != pattern_id
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
                    if let Some(win) = gfx
                        .mini_windows
                        .iter_mut()
                        .find(|w| matches!(w.window_kind, WindowKind::Sequencer))
                    {
                        if !win.is_open {
                            bring_to_front(&mut gfx.z_order, SEQUENCER_ID);
                        }
                        win.is_open = !win.is_open;
                    }
                }
                ClickResult::ToggleMixerWindow => {
                    if let Some(win) = gfx
                        .mini_windows
                        .iter_mut()
                        .find(|w| matches!(w.window_kind, WindowKind::Mixer))
                    {
                        if !win.is_open {
                            bring_to_front(&mut gfx.z_order, MIXER_ID);
                        }

                        win.is_open = !win.is_open;
                    }
                }
                ClickResult::TogglePlaylistWindow => {
                    if let Some(win) = gfx
                        .mini_windows
                        .iter_mut()
                        .find(|w| matches!(w.window_kind, WindowKind::Playlist))
                    {
                        if !win.is_open {
                            bring_to_front(&mut gfx.z_order, PLAYLIST_ID);
                        }
                        win.is_open = !win.is_open;
                    }
                }
                ClickResult::ToggleStep(pattern_id, track_id, step) => {
                    self.producer
                        .try_push(AudioCommand::ToggleStep(pattern_id, track_id, step))
                        .ok();
                    self.project_is_dirty = true;
                }
                ClickResult::Stop => {
                    // resets song back to 0:00

                    // ui state
                    gfx.is_playing = false;
                    gfx.active_step = 0;

                    // audio state
                    self.producer.try_push(AudioCommand::Stop).ok();
                }
                ClickResult::ToggleTrackMute(track_id) => {
                    self.producer
                        .try_push(AudioCommand::ToggleTrackMute(track_id))
                        .ok();
                    self.project_is_dirty = true;
                }
                ClickResult::ChangeBpm(new_bpm) => {
                    self.producer
                        .try_push(AudioCommand::ChangeBpm(new_bpm))
                        .ok();
                    self.project_is_dirty = true;
                }
                ClickResult::TogglePlay => {
                    gfx.is_playing = !gfx.is_playing;

                    self.producer.try_push(AudioCommand::TogglePlay).ok();
                }
                ClickResult::DeleteTrack(track_id) => {
                    self.producer
                        .try_push(AudioCommand::DeleteTrack(track_id))
                        .ok();
                    gfx.tracks.remove(track_id);
                    gfx.mini_windows[SEQUENCER_ID].height =
                        100.0 + TRACK_GAP * gfx.tracks.len() as f32;

                    gfx.context_menu = None;
                    self.project_is_dirty = true;
                }
                ClickResult::ProjectFileDialog => {
                    if self.project_file_dialog_rx.is_none() {
                        let (tx, rx) = std::sync::mpsc::channel::<Option<PathBuf>>();
                        self.project_file_dialog_rx = Some(rx);
                        thread::spawn(move || {
                            let file = FileDialog::new()
                                .add_filter("toml", &["toml"])
                                .set_directory("/")
                                .pick_file();
                            tx.send(file).ok()
                        });
                    }
                }
                ClickResult::TrackFileDialog => {
                    // only allow one track to be added at a time.
                    if self.track_file_dialog_rx.is_none() {
                        // tx -> open file dialog
                        let (tx, rx) = std::sync::mpsc::channel::<Option<PathBuf>>();
                        self.track_file_dialog_rx = Some(rx); // store it
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
                        gfx.context_menu = None
                    }
                }
            }

            // consume the interactions
            self.mouse_state.left_clicked = false;
            self.mouse_state.left_double_clicked = false;
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

                win_attr = win_attr
                    .with_inner_size(winit::dpi::LogicalSize::new(1800, 1200))
                    .with_title("Glacier");
                let window = Rc::new(
                    event_loop
                        .create_window(win_attr)
                        .expect("create window err."),
                );
                pollster::block_on(create_graphics(window, proxy));
            }
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, graphics: Graphics) {
        graphics.request_redraw();
        self.state = State::Ready(Box::new(graphics));
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
            WindowEvent::RedrawRequested => self.draw(event_loop),

            // keyboard event
            WindowEvent::KeyboardInput { event, .. } => {
                if !event.state.is_pressed() {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::ControlLeft) => {
                            self.ctrl_pressed = false;
                        }
                        PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight) => {
                            self.shift_pressed = false;
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
                            self.shift_pressed = true;
                        }
                        PhysicalKey::Code(KeyCode::KeyS) if self.ctrl_pressed => {
                            if let State::Ready(gfx) = &mut self.state {
                                if gfx.project_path
                                    == crate::project::Project::default_project_file()
                                {
                                    // show save-as dialog instead
                                    if self.project_save_dialog_rx.is_none() {
                                        let (tx, rx) =
                                            std::sync::mpsc::channel::<Option<PathBuf>>();
                                        self.project_save_dialog_rx = Some(rx);
                                        thread::spawn(move || {
                                            let file = FileDialog::new()
                                                .add_filter("toml", &["toml"])
                                                .set_file_name("project.toml")
                                                .save_file();
                                            tx.send(file).ok();
                                        });
                                    }
                                } else {
                                    self.producer.try_push(AudioCommand::SaveProject).ok();
                                }
                            }
                        }

                        _ => {}
                    }
                }
            }

            // scroll wheel on mouse
            WindowEvent::MouseWheel { delta, .. } => {
                match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        self.mouse_state.scroll_x = x;
                        self.mouse_state.scroll_y = y;
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        self.mouse_state.scroll_x = pos.x as f32;
                        self.mouse_state.scroll_y = pos.y as f32;
                    }
                }

                if let State::Ready(gfx) = &mut self.state {
                    let scroll_owner = gfx
                        .z_order
                        .iter()
                        .rev()
                        .find(|&&id| {
                            gfx.mini_windows[id].is_open
                                && gfx.mini_windows[id]
                                    .is_hovered(self.mouse_state.x, self.mouse_state.y)
                        })
                        .copied();

                    if scroll_owner == Some(PLAYLIST_ID) {
                        if self.shift_pressed {
                            // stop the piano from moving off the window
                            gfx.playlist_scroll_offset.x = (gfx.playlist_scroll_offset.x
                                - self.mouse_state.scroll_y * 35.0)
                                .clamp(0.0, 1448.0);
                        } else {
                            gfx.playlist_scroll_offset.y = (gfx.playlist_scroll_offset.y
                                - self.mouse_state.scroll_y * 35.0)
                                .clamp(0.0, 1448.0);
                        }
                    } else if scroll_owner == Some(PIANO_ROLL_ID) {
                        if let Some(state) = gfx.piano_roll_state.as_mut() {
                            if self.shift_pressed {
                                state.scroll_offset.x = (state.scroll_offset.x
                                    - self.mouse_state.scroll_y * 35.0)
                                    .clamp(0.0, 1448.0);
                            } else {
                                state.scroll_offset.y = (state.scroll_offset.y
                                    - self.mouse_state.scroll_y * 35.0)
                                    .clamp(0.0, 1448.0);
                            }
                        }
                    } else if scroll_owner == Some(SEQUENCER_ID) {
                        if self.shift_pressed {
                            gfx.sequencer_scroll_offset.x = (gfx.sequencer_scroll_offset.x
                                - self.mouse_state.scroll_y * 35.0)
                                .clamp(0.0, 1448.0);
                        } else {
                            gfx.sequencer_scroll_offset.y = (gfx.sequencer_scroll_offset.y
                                - self.mouse_state.scroll_y * 35.0)
                                .clamp(0.0, 1448.0);
                        }
                    }
                }
            }

            // mouse event
            WindowEvent::MouseInput { state, button, .. } => {
                // left click is PRESSED
                if state.is_pressed() && button == MouseButton::Left {
                    let now = std::time::Instant::now();
                    let is_double_click = self
                        .last_click_time
                        .map(|t| now.duration_since(t).as_millis() < 300)
                        .unwrap_or(false);
                    self.last_click_time = Some(now);

                    if is_double_click {
                        self.mouse_state.left_double_clicked = true;
                    }
                    self.mouse_state.left_click_held = true;
                    self.mouse_state.left_clicked = true;

                    // redraw
                    self.draw(event_loop);
                }
                // left click is RELEASED
                else {
                    self.mouse_state.left_click_held = false;
                    self.mouse_state.left_clicked = false;
                    if let State::Ready(gfx) = &mut self.state {
                        gfx.dragging = false;
                        gfx.dragging_window = None;
                        gfx.dragging_knob = None;
                        gfx.resizing_event = None;
                    }
                }
                if state.is_pressed() && button == MouseButton::Right {
                    // change state
                    self.mouse_state.right_clicked = true;
                    self.right_click_held = true;
                    // redraw
                    self.draw(event_loop);
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
                        match gfx.handle_drag(
                            position.x as f32,
                            position.y as f32,
                            delta_y,
                            delta_x,
                        ) {
                            DragResult::None => {}
                            DragResult::DragMasterVolumeSlider(new_volume) => {
                                self.producer
                                    .try_push(AudioCommand::ChangeMasterVolume(new_volume))
                                    .ok();
                                gfx.request_redraw();
                            }
                            DragResult::DragTrackVolumeKnob(track_id, new_volume) => {
                                self.producer
                                    .try_push(AudioCommand::ChangeTrackVolume(track_id, new_volume))
                                    .ok();
                                gfx.request_redraw();
                            }
                            DragResult::DragTrackVolumeSlider(track_id, new_volume) => {
                                self.producer
                                    .try_push(AudioCommand::ChangeTrackVolume(track_id, new_volume))
                                    .ok();
                                gfx.request_redraw();
                            }
                            DragResult::ResizeAudioBlock(event_id, amount) => {
                                self.producer
                                    .try_push(AudioCommand::ResizeAudioBlock(event_id, amount))
                                    .ok();
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
