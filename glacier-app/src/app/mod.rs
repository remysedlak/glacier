//! main state logic for audio and ui decoupling threads

mod audio_bridge;
pub mod click;
mod events;
mod project_session;
mod rename;
mod runtime_state;
mod ui_command;

use crate::audio::{audio_command::AudioCommand, RetiredAudioData, SpectrumFrame};
use crate::config::UserSettings;
use crate::graphics::mini_window::playlist::PlaylistAction;
use crate::graphics::{
    bring_to_front,
    drag::DragResult,
    mini_window::{PIANO_ROLL_ID, PLAYLIST_ID, SEQUENCER_ID},
    primitives::{RenameState, RenameTarget, PAD_32},
    {create_graphics, Graphics},
};
use crate::project::{
    next_id, spawn_track_load, AudioBlock, AudioBlockID, AudioBlockType, PatternID, Project,
    ProjectSaveJob, ProjectSaveWorker, SavePurpose, TrackID,
};
use crate::COMMAND_RING_CAPACITY;
use audio_bridge::AudioBridge;
pub(super) use audio_bridge::AudioCommandSender;
use cpal::traits::StreamTrait;
use project_session::ProjectSession;
use rfd::FileDialog;
use ringbuf::{
    traits::Consumer,
    {HeapCons, HeapProd},
};
use runtime_state::{FileOperations, InputState, ShutdownState};
use std::collections::HashSet;
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time::Instant;
pub use ui_command::UiCommand;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};

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
    state: GraphicsState,
    config: UserSettings,
    audio: AudioBridge,
    project: ProjectSession,
    input: InputState,
    files: FileOperations,
    shutdown: ShutdownState,
}

enum GraphicsState {
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
        spectrum_producer: HeapProd<SpectrumFrame>,
        spectrum_consumer: HeapCons<SpectrumFrame>,
        retired_audio_producer: HeapProd<RetiredAudioData>,
        retired_audio_consumer: HeapCons<RetiredAudioData>,
        event_loop: &EventLoop<Graphics>,
        config: UserSettings,
        project_file: Option<String>,
    ) -> Self {
        Self {
            state: GraphicsState::Init(Some(event_loop.create_proxy())),
            audio: AudioBridge::new(
                producer,
                audio_consumer,
                ui_producer,
                consumer,
                spectrum_producer,
                spectrum_consumer,
                retired_audio_producer,
                retired_audio_consumer,
            ),
            project: ProjectSession::new(project_file),
            input: InputState::default(),
            files: FileOperations::default(),
            shutdown: ShutdownState::default(),
            config,
        }
    }

    /// if the state is ready, draw each frame and handle it's click results
    fn draw(&mut self, event_loop: &ActiveEventLoop) {
        self.audio.commands.flush_pending();

        if !matches!(self.state, GraphicsState::Ready(_)) {
            return;
        }

        if let Some(result) = self
            .project
            .load_rx
            .as_ref()
            .and_then(|rx| match rx.try_recv() {
                Ok(result) => Some(Some(result)),
                Err(TryRecvError::Disconnected) => Some(None),
                Err(TryRecvError::Empty) => None,
            })
        {
            self.project.load_rx = None;
            if let Some(Ok(loaded)) = result {
                self.project.name = loaded.project.name.clone();
                let (audio_consumer, ui_producer, spectrum_producer, retired_audio_producer) =
                    if let (
                        Some(audio_consumer),
                        Some(ui_producer),
                        Some(spectrum_producer),
                        Some(retired_audio_producer),
                    ) = (
                        self.audio.audio_commands.take(),
                        self.audio.ui_producer.take(),
                        self.audio.spectrum_producer.take(),
                        self.audio.retired_audio_producer.take(),
                    ) {
                        (
                            audio_consumer,
                            ui_producer,
                            spectrum_producer,
                            retired_audio_producer,
                        )
                    } else {
                        use ringbuf::traits::Split;
                        use ringbuf::HeapRb;
                        self.shutdown_audio_stream();
                        let (audio_prod, audio_consumer) =
                            HeapRb::<AudioCommand>::new(COMMAND_RING_CAPACITY).split();
                        let (ui_producer, ui_consumer) =
                            HeapRb::<UiCommand>::new(COMMAND_RING_CAPACITY).split();
                        let (spectrum_producer, spectrum_consumer) =
                            HeapRb::<SpectrumFrame>::new(2).split();
                        let (retired_audio_producer, retired_audio_consumer) =
                            HeapRb::<RetiredAudioData>::new(2).split();
                        self.audio.commands.replace_ring(audio_prod);
                        self.audio.ui_commands = ui_consumer;
                        self.audio.spectrum_consumer = spectrum_consumer;
                        self.audio.retired_audio_consumer = retired_audio_consumer;
                        (
                            audio_consumer,
                            ui_producer,
                            spectrum_producer,
                            retired_audio_producer,
                        )
                    };
                let (stream, sample_rate) = crate::audio::init(
                    audio_consumer,
                    ui_producer,
                    spectrum_producer,
                    retired_audio_producer,
                    loaded.project.clone(),
                    loaded.tracks.clone(),
                );
                self.audio.stream = Some(stream);
                if let GraphicsState::Ready(gfx) = &mut self.state {
                    apply_loaded_project(gfx, loaded);
                    gfx.sample_rate = sample_rate;
                }
                self.project.dirty = false;
                self.project.revision = 0;
            } else if let Some(Err(error)) = result {
                log::error!("failed to load project: {error}");
            }
        }

        // Do not accept project edits until the initial or replacement project is
        // installed. Keep polling the worker via redraw events while it loads.
        if self.project.load_rx.is_some() || self.audio.stream.is_none() {
            if let GraphicsState::Ready(gfx) = &mut self.state {
                gfx.request_redraw();
            }
            return;
        }

        // --- dialogs: only touches gfx fields directly, no self-methods called ---
        if let GraphicsState::Ready(gfx) = &mut self.state {
            // new track loaded
            if let Some(rx) = &self.files.track_dialog_rx {
                match rx.try_recv() {
                    Ok(Some(path)) => {
                        let path_str = path.to_string_lossy().to_string();
                        self.files.track_load_rx = Some(spawn_track_load(path_str));
                        self.files.track_dialog_rx = None;
                    }
                    Ok(None) => self.files.track_dialog_rx = None,
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => self.files.track_dialog_rx = None,
                }
            }
            // new project loaded
            if let Some(rx) = &self.files.project_open_dialog_rx {
                match rx.try_recv() {
                    Ok(Some(path)) => {
                        let path = path.to_string_lossy().to_string();
                        // The pending track belongs to the current project. A
                        // project switch intentionally discards that unfinished
                        // import instead of attaching it to the next project.
                        self.files.track_load_rx = None;
                        self.files.pending_drop = None;
                        if self.project.dirty {
                            let start_switch_save = self.project.pending_path.is_none();
                            self.project.pending_path = Some(path);
                            if start_switch_save {
                                let save_key = (gfx.project_path.clone(), self.project.revision);
                                if !self.project.saves_in_flight.contains(&save_key) {
                                    submit_project_save(
                                        &self.project.save_worker,
                                        &mut self.project.saves_in_flight,
                                        &self.project.name,
                                        gfx,
                                        SavePurpose::ProjectSwitch,
                                        self.project.revision,
                                        None,
                                    );
                                }
                            }
                        } else {
                            self.project.load_rx =
                                Some(crate::project::spawn_project_load(path, false));
                        }
                        self.files.project_open_dialog_rx = None;
                    }
                    Ok(None) => self.files.project_open_dialog_rx = None,
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => self.files.project_open_dialog_rx = None,
                }
            }
            if let Some(rx) = &self.files.project_save_dialog_rx {
                match rx.try_recv() {
                    Ok(Some(path)) => {
                        let path_str = path.to_string_lossy().to_string();
                        if self.project.pending_path.is_none() {
                            submit_project_save(
                                &self.project.save_worker,
                                &mut self.project.saves_in_flight,
                                &self.project.name,
                                gfx,
                                SavePurpose::Regular,
                                self.project.revision,
                                Some(&path_str),
                            );
                        }
                        self.files.project_save_dialog_rx = None;
                    }
                    Ok(None) => self.files.project_save_dialog_rx = None,
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => self.files.project_save_dialog_rx = None,
                }
            }
        } // <- gfx borrow ends here

        // --- drain audio -> ui commands: needs &mut self, so gfx must NOT be held ---
        while let Some(cmd) = self.audio.ui_commands.try_pop() {
            self.handle_ui_command(cmd);
        }

        // Spectrum frames use a small dedicated ring so stale visual updates
        // cannot crowd out transport and meter messages.
        let mut latest_spectrum = None;
        while let Some(frame) = self.audio.spectrum_consumer.try_pop() {
            latest_spectrum = Some(frame);
        }
        if let Some(frame) = latest_spectrum {
            if let GraphicsState::Ready(gfx) = &mut self.state {
                gfx.spectrum.clear();
                gfx.spectrum.extend_from_slice(&frame);
            }
        }

        // Returned playback snapshots and audition buffers are destroyed here,
        // away from the audio callback.
        while let Some(retired) = self.audio.retired_audio_consumer.try_pop() {
            drop(retired);
        }

        loop {
            match self.project.save_worker.try_result() {
                Ok(result) => self.handle_save_result(result, event_loop),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }

        self.poll_track_load();

        if self.shutdown.close_requested {
            if self.files.track_load_rx.is_some() {
                if let GraphicsState::Ready(gfx) = &mut self.state {
                    gfx.request_redraw();
                }
                return;
            }
            self.shutdown.close_requested = false;
            if self.project.dirty {
                if let GraphicsState::Ready(gfx) = &mut self.state {
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

        if self.shutdown.closing {
            if self.files.track_load_rx.is_some() {
                if let GraphicsState::Ready(gfx) = &mut self.state {
                    gfx.request_redraw();
                }
                return;
            }
            if !self.shutdown.close_save_submitted {
                self.shutdown.close_save_submitted = true;
                if !self.project.dirty {
                    crate::config::save(&self.config);
                    self.pause_audio_stream();
                    event_loop.exit();
                    return;
                }
                if let GraphicsState::Ready(gfx) = &self.state {
                    let save_key = (gfx.project_path.clone(), self.project.revision);
                    if !self.project.saves_in_flight.contains(&save_key) {
                        submit_project_save(
                            &self.project.save_worker,
                            &mut self.project.saves_in_flight,
                            &self.project.name,
                            gfx,
                            SavePurpose::Close,
                            self.project.revision,
                            None,
                        );
                    }
                }
            }
            if let GraphicsState::Ready(gfx) = &mut self.state {
                gfx.request_redraw();
            }
            return;
        }

        // --- draw the frame: fresh gfx borrow, scoped tightly ---
        let (result, icon);
        {
            let GraphicsState::Ready(gfx) = &mut self.state else {
                return;
            };

            // if user is currently dragging any UI components
            let any_dragging = gfx.is_dragging();

            let draw_mouse = if any_dragging || gfx.dragging_file.is_some() {
                MouseState {
                    x: if any_dragging {
                        f32::NEG_INFINITY
                    } else {
                        self.input.mouse.x
                    },
                    y: if any_dragging {
                        f32::NEG_INFINITY
                    } else {
                        self.input.mouse.y
                    },
                    left_clicked: false,
                    left_click_held: if gfx.dragging_file.is_some() {
                        false
                    } else {
                        self.input.mouse.left_click_held
                    },
                    ..self.input.mouse
                }
            } else {
                self.input.mouse
            };

            let interaction = gfx.draw(&draw_mouse, self.project.dirty);
            result = interaction.click;
            icon = interaction.cursor;
            gfx.window.set_cursor(icon);
            if gfx.tooltip.is_some() {
                if self.input.mouse.hover_duration.is_none() {
                    self.input.mouse.hover_duration = Some(Instant::now());
                }
            } else {
                self.input.mouse.hover_duration = None;
            }
        } // <- gfx borrow ends here

        // --- handle click result: needs &mut self, so gfx must NOT be held ---
        self.handle_click_result(result, event_loop);

        // clear mouse state after handling click result
        self.input.mouse.clear();

        if let GraphicsState::Ready(gfx) = &mut self.state {
            gfx.request_redraw();
        }
    }

    fn poll_track_load(&mut self) {
        let loaded_track = self
            .files
            .track_load_rx
            .as_ref()
            .and_then(|rx| match rx.try_recv() {
                Ok(loaded) => Some(Some(loaded)),
                Err(TryRecvError::Disconnected) => Some(None),
                Err(TryRecvError::Empty) => None,
            });
        if let Some(loaded_track) = loaded_track {
            self.files.track_load_rx = None;
            if let Some(mut track) = loaded_track {
                if let GraphicsState::Ready(gfx) = &mut self.state {
                    track.data.id =
                        next_id(gfx.tracks.iter().map(|loaded| loaded.data.id.0), TrackID);
                    let track_id = track.data.id;
                    gfx.tracks.push(track.clone());
                    gfx.mini_windows[SEQUENCER_ID].height = 100.0
                        + crate::graphics::mini_window::sequencer::TRACK_GAP
                            * gfx.tracks.len() as f32;
                    gfx.playlist_action =
                        PlaylistAction::PlaceFresh(AudioBlockType::Sample(track_id));
                    let pending_drop = self.files.pending_drop.take();
                    let mut added_block = false;
                    if let Some((playlist_track, step)) = pending_drop {
                        let audio_block = make_audio_block(
                            gfx,
                            playlist_track,
                            step as u32,
                            1,
                            AudioBlockType::Sample(track_id),
                        );
                        gfx.audio_blocks.push(audio_block);
                        added_block = true;
                    } else {
                        gfx.mini_windows[SEQUENCER_ID].is_open = true;
                        bring_to_front(&mut gfx.z_order, SEQUENCER_ID);
                    }
                    click::sync_audio_project(
                        &mut self.audio.commands,
                        gfx,
                        true,
                        false,
                        added_block,
                    );
                    self.project.dirty = true;
                    self.project.revision += 1;
                }
            }
        }
    }

    pub fn shutdown_audio_stream(&mut self) {
        if let Some(stream) = self.audio.stream.take() {
            let _ = stream.pause();
            drop(stream);
        }
    }

    pub(super) fn pause_audio_stream(&self) {
        if let Some(stream) = &self.audio.stream {
            let _ = stream.pause();
        }
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
