//! handle all mouse click interactivity
use super::{App, State};
use crate::app::PianoRollState;
use crate::audio::AudioCommand;
use crate::graphics::{
    bring_to_front,
    context_menu::{ContextMenu, ContextMenuKind},
    mini_window::{
        piano_roll::PIANO_ROLL_DEFAULT_Y, MiniWindow, WindowKind, MIXER_ID, PIANO_ROLL_ID,
        PLAYLIST_ID, SEQUENCER_ID,
    },
    primitives::{RenameState, RenameTarget},
};
use crate::project::AudioBlockType;
use rfd::FileDialog;
use ringbuf::traits::Producer;
use std::path::PathBuf;
use std::thread;
use winit::event_loop::ActiveEventLoop;

/// Each frame, a ClickResult is returned from the draw method. If the mouse left clicked a component on a screen, then a ClickResult is returned and handled in app.rs.
pub enum ClickResult {
    // sequencer
    ToggleStep(usize, u32, usize),     // pattern_id, track_id, step_idx
    ToggleNote(usize, u32, usize, u8), // pattern_id, track_id, step_idx, pitch
    ToggleTrackMute(usize),
    DeleteTrack(usize),
    ToggleSequencerWindow,
    OpenTrackFileLocation(String),

    // toolbar
    Stop,
    ChangeBpm(f32),
    TogglePlay,
    ProjectFileDialog,
    TrackFileDialog,

    // menus
    OpenTrackMenu(f32, f32, usize, usize),
    CloseContextMenu,

    // patterns
    DeletePlaylistAudioBlock(usize),
    DeletePattern(usize),
    DuplicatePattern(usize),
    CreatePattern,
    ClearPattern(usize),
    AddPlaylistAudioBlock(usize, u32, usize, AudioBlockType),
    OpenPatternMenu(f32, f32, usize),
    StartResizeEvent(usize),

    // renaming
    StartRenamingPattern(usize),
    StartRenamingTrack(usize),

    // piano roll
    TogglePianoRollWindow,
    LoadPianoRoll(PianoRollState),

    // toggle ui components
    ToggleMixerWindow,
    TogglePlaylistWindow,
    ToggleTrackWindow(usize),
    TogglePatternTray,
    ToggleTrackTray,
    SelectPattern(usize),
    SelectTrackTray(u32),

    // modal controls
    ModalConfirmSaveAndExit,
    ModalConfirmDiscardAndExit,
    ModalCancelExit,

    // file system
    FsToggleDir(PathBuf),
    FsPreviewSample(PathBuf),
    FsStartDragFile(PathBuf),
    FSEndDragFile(PathBuf, usize, usize), // track, step

    // no click result
    None,
}
impl ClickResult {
    /// combine click results, prioritizing the first if it's not None
    pub fn or(self, other: ClickResult) -> ClickResult {
        if matches!(self, ClickResult::None) {
            other
        } else {
            self
        }
    }
}

impl App {
    pub(super) fn handle_click_result(
        &mut self,
        result: ClickResult,
        _event_loop: &ActiveEventLoop,
    ) {
        let State::Ready(gfx) = &mut self.state else {
            return;
        };

        match result {
            // UI EDITS
            ClickResult::ModalCancelExit => {
                gfx.show_save_modal = false;
            }
            ClickResult::StartRenamingPattern(id) => {
                gfx.context_menu = None;
                if let Some(pattern) = gfx.patterns.iter().find(|p| p.id == id) {
                    gfx.renaming = Some(RenameState {
                        target: RenameTarget::Pattern(id),

                        edited_name: pattern.name.clone(),
                        cursor: pattern.name.len(),
                    });
                }
            }
            ClickResult::StartRenamingTrack(id) => {
                gfx.context_menu = None;
                if let Some(track) = gfx.tracks.iter().find(|t| t.data.id == id as u32) {
                    gfx.renaming = Some(RenameState {
                        target: RenameTarget::Track(id),

                        edited_name: track.data.name.clone(),
                        cursor: track.data.name.len(),
                    });
                }
            }
            ClickResult::FsStartDragFile(path) => gfx.dragging_file = Some(path),
            ClickResult::SelectTrackTray(id) => {
                gfx.active_tray = AudioBlockType::Sample(id as usize)
            }
            ClickResult::FSEndDragFile(path, track, step) => {
                let path_str = path.to_string_lossy().to_string();
                self.pending_drop = Some((track, step));
                self.track_load_rx = Some(crate::project::spawn_track_load(path_str));
            }

            ClickResult::FsToggleDir(path) => {
                if gfx.expanded_dirs.contains(&path) {
                    gfx.expanded_dirs.remove(&path);
                    gfx.fs_cache.remove(&path);
                } else {
                    gfx.expanded_dirs.insert(path.clone());
                    if let Ok(entries) = std::fs::read_dir(&path) {
                        let listing = entries
                            .flatten()
                            .map(|e| {
                                let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                                (e.path(), is_dir)
                            })
                            .collect();
                        gfx.fs_cache.insert(path, listing);
                    }
                }
            }
            ClickResult::TogglePatternTray => gfx.show_pattern_tray = !gfx.show_pattern_tray,
            ClickResult::ToggleTrackTray => gfx.show_track_tray = !gfx.show_track_tray,
            ClickResult::OpenTrackFileLocation(path) => {
                let abs = std::fs::canonicalize(&path).unwrap_or_else(|_| PathBuf::from(&path));
                thread::spawn(move || {
                    showfile::show_path_in_file_manager(abs);
                });
            }
            ClickResult::StartResizeEvent(id) => gfx.resizing_event = Some(id),

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
            ClickResult::CloseContextMenu => gfx.context_menu = None,
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

            ClickResult::SelectPattern(pattern_id) => {
                gfx.active_pattern_id = pattern_id;
                gfx.active_tray = AudioBlockType::Pattern(pattern_id);
            }
            ClickResult::ToggleTrackWindow(track) => {
                gfx.piano_roll_state = Some(super::PianoRollState {
                    pattern_id: gfx.active_pattern_id,
                    track_id: gfx.tracks[track].data.id,
                    scroll_offset: super::ScrollOffset {
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
            // AUDIO PING
            ClickResult::ClearPattern(pattern_id) => {
                self.producer
                    .try_push(AudioCommand::ClearPattern(pattern_id))
                    .ok();
                gfx.context_menu = None;
            }
            ClickResult::ModalConfirmSaveAndExit => {
                gfx.show_save_modal = false;
                self.producer.try_push(AudioCommand::Shutdown).ok();
            }
            ClickResult::ModalConfirmDiscardAndExit => {
                gfx.show_save_modal = false;
                self.producer
                    .try_push(AudioCommand::ShutdownWithoutSaving)
                    .ok();
            }
            // play a 5 second sample of the audio clip
            ClickResult::FsPreviewSample(track_path) => {
                let preview = crate::project::path_to_preview(&track_path.to_string_lossy(), 5);
                self.producer
                    .try_push(AudioCommand::PreviewSample(preview))
                    .ok();
            }
            ClickResult::ChangeBpm(bpm) => {
                self.producer.try_push(AudioCommand::ChangeBpm(bpm)).ok();
            }
            ClickResult::DuplicatePattern(pattern_id) => {
                self.producer
                    .try_push(AudioCommand::DuplicatePattern(pattern_id))
                    .ok();
            }
            ClickResult::ToggleNote(pattern_id, track_id, step_idx, pitch) => {
                self.producer
                    .try_push(AudioCommand::ToggleNote(
                        pattern_id, track_id, step_idx, pitch,
                    ))
                    .ok();
            }
            ClickResult::DeletePlaylistAudioBlock(id) => {
                self.producer
                    .try_push(AudioCommand::DeleteAudioBlock(id))
                    .ok();
            }
            ClickResult::AddPlaylistAudioBlock(track, start_step, length, block_type) => {
                // request audio thread to create audio block
                self.producer
                    .try_push(AudioCommand::CreateAudioBlock(
                        track,
                        start_step,
                        length,
                        block_type.clone(),
                    ))
                    .ok();
            }
            ClickResult::DeletePattern(pattern_id) => {
                self.producer
                    .try_push(AudioCommand::DeletePattern(pattern_id))
                    .ok();
            }
            ClickResult::ToggleStep(pattern_id, track_id, step) => {
                self.producer
                    .try_push(AudioCommand::ToggleStep(pattern_id, track_id, step))
                    .ok();
            }
            ClickResult::Stop => {
                self.producer.try_push(AudioCommand::Stop).ok();
            }
            ClickResult::ToggleTrackMute(track_id) => {
                self.producer
                    .try_push(AudioCommand::ToggleTrackMute(track_id))
                    .ok();
            }
            ClickResult::TogglePlay => {
                self.producer.try_push(AudioCommand::TogglePlay).ok();
            }
            ClickResult::DeleteTrack(track_id) => {
                self.producer
                    .try_push(AudioCommand::DeleteTrack(track_id))
                    .ok();
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
                if self.track_file_dialog_rx.is_none() {
                    let (tx, rx) = std::sync::mpsc::channel::<Option<PathBuf>>();
                    self.track_file_dialog_rx = Some(rx);
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
    }
}
