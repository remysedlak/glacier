//! main state logic for audio and ui decoupling threads
use crate::audio::audio_command::AudioCommand;
use crate::config::UserSettings;
use crate::graphics::{
    bring_to_front,
    drag::DragResult,
    mini_window::{PIANO_ROLL_ID, PLAYLIST_ID, SEQUENCER_ID},
    primitives::{RenameState, RenameTarget, PAD_32},
    PlaylistAction, {create_graphics, Graphics, Rc},
};
use crate::project::{
    next_id, spawn_track_load, AudioBlock, AudioBlockID, AudioBlockType, PatternID, Project,
    ProjectSaveJob, ProjectSaveWorker, SavePurpose, Track, TrackID,
};
use cpal::{traits::StreamTrait, Stream};
use rfd::FileDialog;
use ringbuf::{
    traits::{Consumer, Producer},
    {HeapCons, HeapProd},
};
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread;
use std::time::Instant;
use winit::window::Icon;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

pub mod click;
mod rename;
mod ui_command;

pub use ui_command::UiCommand;

#[derive(Clone, Copy, Debug)]
pub struct MouseState {
    pub x: f32,
    pub y: f32,
    pub left_clicked: bool,
    pub left_double_clicked: bool,
    pub left_click_held: bool,
    pub right_clicked: bool,
    pub left_released: bool,
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub hover_duration: Option<Instant>,
}
impl Default for MouseState {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            left_clicked: false,
            left_double_clicked: false,
            right_clicked: false,
            scroll_x: 0.0,
            scroll_y: 0.0,
            left_click_held: false,
            left_released: false,
            hover_duration: None,
        }
    }
}

impl MouseState {
    pub fn clear(&mut self) {
        self.left_clicked = false;
        self.left_double_clicked = false;
        self.right_clicked = false;
        self.scroll_x = 0.0;
        self.scroll_y = 0.0;
        self.left_released = false;
    }

    /// Returns a copy with position/click fields blanked so downstream
    /// hit-tests can't register anything — used when something else
    /// visually owns this screen area right now.
    pub fn hidden(self, hide: bool) -> MouseState {
        if hide {
            MouseState {
                x: f32::NEG_INFINITY,
                y: f32::NEG_INFINITY,
                left_clicked: false,
                right_clicked: false,
                left_double_clicked: false,
                left_click_held: false,
                ..self
            }
        } else {
            self
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ScrollOffset {
    pub x: f32,
    pub y: f32,
}
impl Default for ScrollOffset {
    fn default() -> Self {
        ScrollOffset { x: 0.0, y: 0.0 }
    }
}

#[derive(PartialEq, Debug)]
pub struct PianoRollState {
    pub pattern_id: PatternID,
    pub track_id: TrackID,
    pub scroll_offset: ScrollOffset,
}

pub struct App {
    producer: AudioCommandSender,
    consumer: HeapCons<UiCommand>,
    state: State,
    config: UserSettings,
    stream: Option<Stream>,
    audio_consumer: Option<HeapCons<AudioCommand>>,
    ui_producer: Option<HeapProd<UiCommand>>,
    project_load_rx: Option<Receiver<Result<crate::project::ProjectLoadResult, String>>>,
    save_worker: ProjectSaveWorker,
    project_name: String,
    pending_project: Option<String>,
    close_requested: bool,
    is_closing: bool,
    close_save_submitted: bool,
    regular_saves_in_flight: HashSet<(String, u64)>,
    project_is_dirty: bool,
    project_revision: u64,
    ctrl_pressed: bool,
    pub shift_pressed: bool,
    mouse_state: MouseState,
    right_click_held: bool,
    last_click_time: Option<std::time::Instant>,
    track_file_dialog_rx: Option<Receiver<Option<PathBuf>>>,
    project_file_dialog_rx: Option<Receiver<Option<PathBuf>>>,
    track_load_rx: Option<Receiver<Track>>,
    project_save_dialog_rx: Option<Receiver<Option<PathBuf>>>,
    pending_drop: Option<(u32, usize)>,
}

/// UI-side sender that retains commands when the audio ring buffer is full.
/// Commands stay ordered and are retried on subsequent app events/draws.
pub(crate) struct AudioCommandSender {
    ring: HeapProd<AudioCommand>,
    pending: VecDeque<AudioCommand>,
}

impl AudioCommandSender {
    fn new(ring: HeapProd<AudioCommand>) -> Self {
        Self {
            ring,
            pending: VecDeque::new(),
        }
    }

    pub(crate) fn try_push(&mut self, command: AudioCommand) -> Result<(), AudioCommand> {
        if !self.pending.is_empty() {
            self.pending.push_back(command);
            return Ok(());
        }

        match self.ring.try_push(command) {
            Ok(()) => Ok(()),
            Err(command) => {
                self.pending.push_back(command);
                Ok(())
            }
        }
    }

    pub(crate) fn flush_pending(&mut self) {
        while let Some(command) = self.pending.pop_front() {
            if let Err(command) = self.ring.try_push(command) {
                self.pending.push_front(command);
                break;
            }
        }
    }

    fn replace_ring(&mut self, ring: HeapProd<AudioCommand>) {
        self.pending.clear();
        self.ring = ring;
    }
}

enum State {
    Ready(Box<Graphics>),
    Init(Option<EventLoopProxy<Graphics>>),
}

impl App {
    /// Initialzie the app's state
    pub fn new(
        producer: HeapProd<AudioCommand>,
        audio_consumer: HeapCons<AudioCommand>,
        ui_producer: HeapProd<UiCommand>,
        consumer: HeapCons<UiCommand>,
        event_loop: &EventLoop<Graphics>,
        config: UserSettings,
        project_file: Option<String>,
    ) -> Self {
        let save_worker = ProjectSaveWorker::new();
        let project_path =
            project_file.unwrap_or_else(crate::project::Project::default_project_file);
        let project_load_rx = Some(crate::project::spawn_project_load(project_path, true));

        Self {
            producer: AudioCommandSender::new(producer),
            consumer,
            state: State::Init(Some(event_loop.create_proxy())),
            stream: None,
            audio_consumer: Some(audio_consumer),
            ui_producer: Some(ui_producer),
            project_load_rx,
            save_worker,
            project_name: String::new(),
            pending_project: None,
            close_requested: false,
            is_closing: false,
            close_save_submitted: false,
            regular_saves_in_flight: HashSet::new(),
            project_is_dirty: false,
            project_revision: 0,
            ctrl_pressed: false,
            shift_pressed: false,
            track_file_dialog_rx: None,
            project_file_dialog_rx: None,
            project_save_dialog_rx: None,
            track_load_rx: None,
            last_click_time: None,
            pending_drop: None,
            config,
            right_click_held: false,
            mouse_state: MouseState::default(),
        }
    }

    /// if the state is ready, draw each frame and handle it's click results
    fn draw(&mut self, event_loop: &ActiveEventLoop) {
        self.producer.flush_pending();

        if !matches!(self.state, State::Ready(_)) {
            return;
        }

        if let Some(result) = self
            .project_load_rx
            .as_ref()
            .and_then(|rx| match rx.try_recv() {
                Ok(result) => Some(Some(result)),
                Err(TryRecvError::Disconnected) => Some(None),
                Err(TryRecvError::Empty) => None,
            })
        {
            self.project_load_rx = None;
            if let Some(Ok(loaded)) = result {
                self.project_name = loaded.project.name.clone();
                let (audio_consumer, ui_producer) =
                    if let (Some(audio_consumer), Some(ui_producer)) =
                        (self.audio_consumer.take(), self.ui_producer.take())
                    {
                        (audio_consumer, ui_producer)
                    } else {
                        use ringbuf::traits::Split;
                        use ringbuf::HeapRb;
                        self.shutdown_audio_stream();
                        let (audio_prod, audio_consumer) = HeapRb::<AudioCommand>::new(256).split();
                        let (ui_producer, ui_consumer) = HeapRb::<UiCommand>::new(256).split();
                        self.producer.replace_ring(audio_prod);
                        self.consumer = ui_consumer;
                        (audio_consumer, ui_producer)
                    };
                let (stream, sample_rate) = crate::audio::init(
                    audio_consumer,
                    ui_producer,
                    loaded.project.clone(),
                    loaded.tracks.clone(),
                );
                self.stream = Some(stream);
                if let State::Ready(gfx) = &mut self.state {
                    apply_loaded_project(gfx, loaded);
                    gfx.sample_rate = sample_rate;
                }
                self.project_is_dirty = false;
                self.project_revision = 0;
            } else if let Some(Err(error)) = result {
                log::error!("failed to load project: {error}");
            }
        }

        // Do not accept project edits until the initial or replacement project is
        // installed. Keep polling the worker via redraw events while it loads.
        if self.project_load_rx.is_some() || self.stream.is_none() {
            if let State::Ready(gfx) = &mut self.state {
                gfx.request_redraw();
            }
            return;
        }

        // --- dialogs: only touches gfx fields directly, no self-methods called ---
        if let State::Ready(gfx) = &mut self.state {
            // new track loaded
            if let Some(rx) = &self.track_file_dialog_rx {
                match rx.try_recv() {
                    Ok(Some(path)) => {
                        let path_str = path.to_string_lossy().to_string();
                        self.track_load_rx = Some(spawn_track_load(path_str));
                        self.track_file_dialog_rx = None;
                    }
                    Ok(None) => self.track_file_dialog_rx = None,
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => self.track_file_dialog_rx = None,
                }
            }
            // new project loaded
            if let Some(rx) = &self.project_file_dialog_rx {
                match rx.try_recv() {
                    Ok(Some(path)) => {
                        let path = path.to_string_lossy().to_string();
                        // The pending track belongs to the current project. A
                        // project switch intentionally discards that unfinished
                        // import instead of attaching it to the next project.
                        self.track_load_rx = None;
                        self.pending_drop = None;
                        if self.project_is_dirty {
                            let start_switch_save = self.pending_project.is_none();
                            self.pending_project = Some(path);
                            if start_switch_save {
                                let save_key = (gfx.project_path.clone(), self.project_revision);
                                if !self.regular_saves_in_flight.contains(&save_key) {
                                    submit_project_save(
                                        &self.save_worker,
                                        &mut self.regular_saves_in_flight,
                                        &self.project_name,
                                        gfx,
                                        SavePurpose::ProjectSwitch,
                                        self.project_revision,
                                        None,
                                    );
                                }
                            }
                        } else {
                            self.project_load_rx =
                                Some(crate::project::spawn_project_load(path, false));
                        }
                        self.project_file_dialog_rx = None;
                    }
                    Ok(None) => self.project_file_dialog_rx = None,
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => self.project_file_dialog_rx = None,
                }
            }
            if let Some(rx) = &self.project_save_dialog_rx {
                match rx.try_recv() {
                    Ok(Some(path)) => {
                        let path_str = path.to_string_lossy().to_string();
                        if self.pending_project.is_none() {
                            submit_project_save(
                                &self.save_worker,
                                &mut self.regular_saves_in_flight,
                                &self.project_name,
                                gfx,
                                SavePurpose::Regular,
                                self.project_revision,
                                Some(&path_str),
                            );
                        }
                        self.project_save_dialog_rx = None;
                    }
                    Ok(None) => self.project_save_dialog_rx = None,
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => self.project_save_dialog_rx = None,
                }
            }
        } // <- gfx borrow ends here

        // --- drain audio -> ui commands: needs &mut self, so gfx must NOT be held ---
        while let Some(cmd) = self.consumer.try_pop() {
            self.handle_ui_command(cmd);
        }

        loop {
            match self.save_worker.try_result() {
                Ok(result) => self.handle_save_result(result, event_loop),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }

        self.poll_track_load();

        if self.close_requested {
            if self.track_load_rx.is_some() {
                if let State::Ready(gfx) = &mut self.state {
                    gfx.request_redraw();
                }
                return;
            }
            self.close_requested = false;
            if self.project_is_dirty {
                if let State::Ready(gfx) = &mut self.state {
                    gfx.show_save_modal = true;
                    gfx.request_redraw();
                }
            } else {
                crate::config::save(&self.config);
                self.pause_audio_stream();
                event_loop.exit();
                return;
            }
        }

        if self.is_closing {
            if self.track_load_rx.is_some() {
                if let State::Ready(gfx) = &mut self.state {
                    gfx.request_redraw();
                }
                return;
            }
            if !self.close_save_submitted {
                self.close_save_submitted = true;
                if !self.project_is_dirty {
                    crate::config::save(&self.config);
                    self.pause_audio_stream();
                    event_loop.exit();
                    return;
                }
                if let State::Ready(gfx) = &self.state {
                    let save_key = (gfx.project_path.clone(), self.project_revision);
                    if !self.regular_saves_in_flight.contains(&save_key) {
                        submit_project_save(
                            &self.save_worker,
                            &mut self.regular_saves_in_flight,
                            &self.project_name,
                            gfx,
                            SavePurpose::Close,
                            self.project_revision,
                            None,
                        );
                    }
                }
            }
            if let State::Ready(gfx) = &mut self.state {
                gfx.request_redraw();
            }
            return;
        }

        // --- draw the frame: fresh gfx borrow, scoped tightly ---
        let (result, icon);
        {
            let State::Ready(gfx) = &mut self.state else {
                return;
            };

            let start = std::time::Instant::now();
            // if user is currently dragging any UI components
            let any_dragging = gfx.is_dragging();

            let draw_mouse = if any_dragging || gfx.dragging_file.is_some() {
                MouseState {
                    x: if any_dragging {
                        f32::NEG_INFINITY
                    } else {
                        self.mouse_state.x
                    },
                    y: if any_dragging {
                        f32::NEG_INFINITY
                    } else {
                        self.mouse_state.y
                    },
                    left_clicked: false,
                    left_click_held: if gfx.dragging_file.is_some() {
                        false
                    } else {
                        self.mouse_state.left_click_held
                    },
                    ..self.mouse_state
                }
            } else {
                self.mouse_state
            };

            let interaction = gfx.draw(&draw_mouse, self.project_is_dirty);
            result = interaction.click;
            icon = interaction.cursor;
            gfx.window.set_cursor(icon);
            gfx.frame_ms = start.elapsed().as_secs_f32() * 1000.0;

            if gfx.tooltip.is_some() {
                if self.mouse_state.hover_duration.is_none() {
                    self.mouse_state.hover_duration = Some(Instant::now());
                }
            } else {
                self.mouse_state.hover_duration = None;
            }
        } // <- gfx borrow ends here

        // --- handle click result: needs &mut self, so gfx must NOT be held ---
        self.handle_click_result(result, event_loop);

        // clear mouse state after handling click result
        self.mouse_state.clear();

        if let State::Ready(gfx) = &mut self.state {
            gfx.request_redraw();
        }
    }

    /// Response to user resizing the main window
    fn resized(&mut self, size: PhysicalSize<u32>) {
        if let State::Ready(gfx) = &mut self.state {
            gfx.resize(size);
        }
    }

    fn poll_track_load(&mut self) {
        let loaded_track = self
            .track_load_rx
            .as_ref()
            .and_then(|rx| match rx.try_recv() {
                Ok(loaded) => Some(Some(loaded)),
                Err(TryRecvError::Disconnected) => Some(None),
                Err(TryRecvError::Empty) => None,
            });
        if let Some(loaded_track) = loaded_track {
            self.track_load_rx = None;
            if let Some(mut track) = loaded_track {
                if let State::Ready(gfx) = &mut self.state {
                    track.data.id =
                        next_id(gfx.tracks.iter().map(|loaded| loaded.data.id.0), TrackID);
                    let track_id = track.data.id;
                    gfx.tracks.push(track.clone());
                    gfx.mini_windows[SEQUENCER_ID].height = 100.0
                        + crate::graphics::mini_window::sequencer::TRACK_GAP
                            * gfx.tracks.len() as f32;
                    gfx.playlist_action =
                        PlaylistAction::PlaceFresh(AudioBlockType::Sample(track_id));
                    self.producer.try_push(AudioCommand::LoadTrack(track)).ok();
                    let pending_drop = self.pending_drop.take();
                    if let Some((playlist_track, step)) = pending_drop {
                        let audio_block = make_audio_block(
                            gfx,
                            playlist_track,
                            step as u32,
                            1,
                            AudioBlockType::Sample(track_id),
                        );
                        gfx.audio_blocks.push(audio_block.clone());
                        self.producer
                            .try_push(AudioCommand::CreateAudioBlock(audio_block))
                            .ok();
                    } else {
                        gfx.mini_windows[SEQUENCER_ID].is_open = true;
                        bring_to_front(&mut gfx.z_order, SEQUENCER_ID);
                    }
                    self.project_is_dirty = true;
                    self.project_revision += 1;
                }
            }
        }
    }

    pub fn shutdown_audio_stream(&mut self) {
        if let Some(stream) = self.stream.take() {
            let _ = stream.pause();
            drop(stream);
        }
    }

    pub(super) fn pause_audio_stream(&self) {
        if let Some(stream) = &self.stream {
            let _ = stream.pause();
        }
    }
}

fn load_icon() -> Icon {
    let bytes = include_bytes!("../../../assets/glacier_png.png");
    let image = image::load_from_memory(bytes)
        .expect("failed to decode icon")
        .into_rgba8();
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();
    Icon::from_rgba(rgba, width, height).expect("failed to create icon")
}

use winit::platform::wayland::WindowAttributesExtWayland;
impl ApplicationHandler<Graphics> for App {
    /// typically fires once at start for OS
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let State::Init(proxy) = &mut self.state {
            if let Some(proxy) = proxy.take() {
                let mut win_attr = Window::default_attributes();
                let icon = load_icon();
                win_attr = win_attr
                    .with_inner_size(winit::dpi::LogicalSize::new(1800, 1200))
                    .with_window_icon(Some(icon)) // windows icon (wayland attaches icon at .desktop, mac uses embeded .icns )
                    .with_title("Glacier") // window decoration title, cross-platform, already confirmed working
                    .with_name("Glacier", ""); // wayland app_id / taskbar label

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
        let graphics = graphics;
        graphics.request_redraw();
        self.state = State::Ready(Box::new(graphics));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Destroy native audio and graphics resources while winit's event loop
        // is still alive. In particular, some window/surface backends depend on
        // the event loop's platform state during destruction.
        self.shutdown_audio_stream();
        self.state = State::Init(None);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if (self.close_requested || self.is_closing)
            && !matches!(
                event,
                WindowEvent::CloseRequested | WindowEvent::RedrawRequested
            )
        {
            return;
        }
        match event {
            // User clicks the (x) button on the app [soft shutdown]
            WindowEvent::CloseRequested => {
                if self.is_closing || self.close_requested {
                    return;
                }
                if self.stream.is_none() {
                    crate::config::save(&self.config);
                    event_loop.exit();
                    return;
                }
                // Cancel an in-flight project switch and close the currently open
                // project. Dropping the receiver makes the worker discard its result.
                if self.project_load_rx.is_some() {
                    self.project_load_rx = None;
                    self.pending_project = None;
                }
                self.close_requested = true;
                if let State::Ready(gfx) = &mut self.state {
                    gfx.request_redraw();
                }
            }
            // resizing the main app window
            WindowEvent::Resized(size) => self.resized(size),
            // draw the next frame
            WindowEvent::RedrawRequested => self.draw(event_loop),
            // manage keyboard presses
            WindowEvent::KeyboardInput { event, .. } => {
                if self.project_load_rx.is_some() || self.stream.is_none() {
                    return;
                }
                if !event.state.is_pressed() {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::ControlLeft) => self.ctrl_pressed = false,
                        PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight) => {
                            self.shift_pressed = false;
                        }
                        _ => {}
                    }
                }

                if event.state.is_pressed() {
                    // handle rename state and keyboard actions first
                    if self.handle_rename_key(&event) {
                        return;
                    }
                    if let State::Ready(gfx) = &mut self.state {
                        match event.physical_key {
                            PhysicalKey::Code(KeyCode::Space) => {
                                gfx.is_playing = !gfx.is_playing;
                                gfx.request_redraw();
                                self.producer.try_push(AudioCommand::TogglePlay).ok();
                            }
                            PhysicalKey::Code(KeyCode::ControlLeft) => self.ctrl_pressed = true,
                            PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight) => {
                                self.shift_pressed = true;
                            }
                            PhysicalKey::Code(KeyCode::F2) => {
                                if let PlaylistAction::PlaceFresh(AudioBlockType::Pattern(id)) =
                                    gfx.playlist_action
                                {
                                    if let Some(pattern) = gfx.patterns.iter().find(|p| p.id == id)
                                    {
                                        gfx.renaming = Some(RenameState {
                                            target: RenameTarget::Pattern(id),
                                            edited_name: pattern.name.clone(),
                                            cursor: pattern.name.len(),
                                        });
                                    }
                                }
                            }
                            PhysicalKey::Code(KeyCode::KeyS)
                                if self.ctrl_pressed
                                    && self.pending_project.is_none()
                                    && (self.project_is_dirty || project_needs_save_as(gfx)) =>
                            {
                                if project_needs_save_as(gfx) {
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
                                    submit_project_save(
                                        &self.save_worker,
                                        &mut self.regular_saves_in_flight,
                                        &self.project_name,
                                        gfx,
                                        SavePurpose::Regular,
                                        self.project_revision,
                                        None,
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            // manage mouse wheel scrolls (up and down)
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

                    // PLAYLIST SCROLLING
                    if scroll_owner == Some(PLAYLIST_ID) {
                        if self.shift_pressed {
                            gfx.playlist_scroll_offset.x = (gfx.playlist_scroll_offset.x
                                - self.mouse_state.scroll_y * 35.0)
                                .clamp(0.0, 1448.0);
                        } else {
                            gfx.playlist_scroll_offset.y = (gfx.playlist_scroll_offset.y
                                - self.mouse_state.scroll_y * 35.0)
                                .clamp(0.0, 1448.0);
                        }
                    }
                    // PIANO ROLL SCROLLING
                    else if scroll_owner == Some(PIANO_ROLL_ID) {
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
                    }
                    // SEQUENCER SCROLLING
                    else if scroll_owner == Some(SEQUENCER_ID) {
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
                    // TRACK TRAY SCROLLING
                    else if self.mouse_state.x < gfx.track_tray_width
                        && self.mouse_state.y
                            > (gfx.render_context.surface_config.height as f32 / 2.0)
                    {
                        let divider_y = gfx.render_context.surface_config.height as f32 / 2.0;
                        let visible_height =
                            gfx.render_context.surface_config.height as f32 - divider_y;
                        let total_rows = crate::project::count_fs_rows(
                            &gfx.user_fs_location,
                            &gfx.expanded_dirs,
                            &gfx.fs_cache,
                        );
                        let content_height = total_rows as f32 * PAD_32;
                        let max_scroll = (content_height - visible_height).max(0.0);
                        gfx.fs_scroll_offset = (gfx.fs_scroll_offset
                            - self.mouse_state.scroll_y * 20.0)
                            .clamp(0.0, max_scroll);
                        gfx.request_redraw();
                    }
                }
            }
            // manage mouse clicks (left + right)
            WindowEvent::MouseInput { state, button, .. } => {
                // LEFT CLICK
                if state.is_pressed() && button == MouseButton::Left {
                    // check if part of a double click
                    let now = std::time::Instant::now();
                    let is_double_click = self
                        .last_click_time
                        .map(|t| now.duration_since(t).as_millis() < 300)
                        .unwrap_or(false);
                    self.last_click_time = Some(now);
                    if is_double_click {
                        self.mouse_state.left_double_clicked = true;
                    }
                    self.mouse_state.left_clicked = true;
                    self.mouse_state.left_click_held = true;

                    self.draw(event_loop);
                }
                // NOT LEFT CLICK
                else {
                    self.mouse_state.left_released = true;
                    self.mouse_state.left_click_held = false;
                    self.mouse_state.left_clicked = false;
                    self.draw(event_loop);

                    if let State::Ready(gfx) = &mut self.state {
                        gfx.clear_drag_state();
                    }
                }
                // RIGHT CLICK
                if state.is_pressed() && button == MouseButton::Right {
                    self.mouse_state.right_clicked = true;
                    self.right_click_held = true;
                    self.draw(event_loop);
                }
                // NO RIGHT CLICK
                else {
                    self.right_click_held = false;
                    self.mouse_state.right_clicked = false;
                }
            }
            // manage mouse movement (drag / drop + hover)
            WindowEvent::CursorMoved { position, .. } => {
                // calculate mouse distance
                let delta_y = position.y as f32 - self.mouse_state.y;
                let delta_x = position.x as f32 - self.mouse_state.x;
                // update mouse state
                self.mouse_state.x = position.x as f32;
                self.mouse_state.y = position.y as f32;

                if let State::Ready(gfx) = &mut self.state {
                    // IF USER IS ALREADY DRAGGING SOMETHING
                    if self.mouse_state.left_click_held {
                        match gfx.handle_drag(
                            position.x as f32,
                            position.y as f32,
                            delta_y,
                            delta_x,
                        ) {
                            DragResult::None => {}
                            DragResult::DraggingFile => gfx.request_redraw(),
                            DragResult::ResizeTrackTray => gfx.request_redraw(),
                            DragResult::DragMasterVolumeSlider(new_volume) => {
                                self.producer
                                    .try_push(AudioCommand::ChangeMasterVolume(new_volume))
                                    .ok();
                                self.project_is_dirty = true;
                                self.project_revision += 1;
                                gfx.request_redraw();
                            }
                            DragResult::DragTrackVolumeKnob(track_id, new_volume) => {
                                self.producer
                                    .try_push(AudioCommand::ChangeTrackVolume(track_id, new_volume))
                                    .ok();
                                self.project_is_dirty = true;
                                self.project_revision += 1;
                                gfx.request_redraw();
                            }
                            DragResult::DragTrackVolumeSlider(track_id, new_volume) => {
                                self.producer
                                    .try_push(AudioCommand::ChangeTrackVolume(track_id, new_volume))
                                    .ok();
                                self.project_is_dirty = true;
                                self.project_revision += 1;
                                gfx.request_redraw();
                            }
                            DragResult::ResizeAudioBlock(event_id, amount) => {
                                self.producer
                                    .try_push(AudioCommand::ResizeAudioBlock(event_id, amount))
                                    .ok();
                                self.project_is_dirty = true;
                                self.project_revision += 1;
                            }
                        }
                    } else {
                        gfx.clear_drag_state()
                    }
                }
            }
            _ => {}
        }
        self.producer.flush_pending();
    }
}

pub(super) fn submit_project_save(
    worker: &ProjectSaveWorker,
    regular_saves_in_flight: &mut HashSet<(String, u64)>,
    project_name: &str,
    gfx: &Graphics,
    purpose: SavePurpose,
    revision: u64,
    path_override: Option<&str>,
) {
    let project = Project::new(
        project_name.to_owned(),
        gfx.bpm,
        gfx.master_volume,
        &gfx.tracks,
        gfx.patterns.clone(),
        gfx.audio_blocks.clone(),
    );
    let path = path_override.unwrap_or(&gfx.project_path).to_owned();
    if purpose == SavePurpose::Regular {
        let key = (path.clone(), revision);
        if regular_saves_in_flight.contains(&key) {
            return;
        }
        regular_saves_in_flight.insert(key);
    }
    let job = ProjectSaveJob {
        project,
        path: path.clone(),
        purpose,
        revision,
    };
    if let Err(error) = worker.submit(job) {
        if purpose == SavePurpose::Regular {
            regular_saves_in_flight.remove(&(path, revision));
        }
        log::error!("could not queue project save: {error}");
    }
}

/// A project without an existing destination needs Save As. This covers the
/// empty path used by a new project, the default starter path, and a new
/// project path before its first save.
fn project_needs_save_as(gfx: &Graphics) -> bool {
    gfx.project_path.is_empty()
        || gfx.project_path == crate::project::Project::default_project_file()
        || !std::path::Path::new(&gfx.project_path).exists()
}

pub(super) fn apply_loaded_project(gfx: &mut Graphics, loaded: crate::project::ProjectLoadResult) {
    gfx.is_playing = false;
    gfx.active_step = 0;
    gfx.playhead_beat = 0.0;
    gfx.tracks = loaded.tracks;
    gfx.patterns = loaded.project.patterns;
    gfx.audio_blocks = loaded.project.audio_blocks;
    gfx.bpm = loaded.project.bpm;
    gfx.master_volume = loaded.project.master_volume;
    gfx.project_path = loaded.path;
    gfx.mini_windows[SEQUENCER_ID].height =
        100.0 + crate::graphics::mini_window::sequencer::TRACK_GAP * gfx.tracks.len() as f32;
}

pub(super) fn make_audio_block(
    gfx: &Graphics,
    track: u32,
    start_step: u32,
    length: u32,
    block_type: AudioBlockType,
) -> AudioBlock {
    let length = match block_type {
        AudioBlockType::Sample(sample_track_id) => gfx
            .tracks
            .iter()
            .find(|track| track.data.id == sample_track_id)
            .map(|loaded_track| {
                let step_frames = glacier_dsp::utils::samples_per_step(gfx.sample_rate, gfx.bpm);
                let frames =
                    (loaded_track.samples.len() / loaded_track.data.channels as usize) as f32;
                (frames / step_frames).ceil() as u32
            })
            .unwrap_or(length),
        _ => length,
    };
    AudioBlock {
        id: next_id(
            gfx.audio_blocks.iter().map(|block| block.id.0),
            AudioBlockID,
        ),
        track,
        start_step,
        length,
        block_type,
        is_muted: false,
    }
}
