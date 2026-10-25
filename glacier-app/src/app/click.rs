//! handle all mouse click interactivity
use super::{App, State};
use crate::app::PianoRollState;
use crate::audio::audio_command::AudioCommand;
use crate::graphics::PlaylistAction;
use crate::graphics::{
    bring_to_front,
    components::context_menu::{ContextMenu, ContextMenuKind},
    mini_window::{
        piano_roll::PIANO_ROLL_DEFAULT_Y, playlist::toolbar::PlaylistTool, MiniWindow, WindowKind,
        MIXER_ID, PIANO_ROLL_ID, PLAYLIST_ID, SEQUENCER_ID,
    },
    primitives::{RenameState, RenameTarget},
};
use crate::project::{
    next_id, AudioBlockID, AudioBlockType, Note, PatternData, PatternID, Sequence, TrackID,
};
use rfd::FileDialog;
use std::path::PathBuf;
use std::thread;
use winit::event_loop::ActiveEventLoop;

/// Each frame, a ClickResult is returned from the draw method. If the mouse left clicked a component on a screen, then a ClickResult is returned and handled in app.rs.
#[derive(PartialEq, Debug)]
pub enum ClickResult {
    // sequencer
    ToggleStep(PatternID, TrackID, usize), // pattern_id, track_id, step_idx
    ToggleNote(PatternID, TrackID, usize, u8), // pattern_id, track_id, step_idx, pitch
    ToggleTrackMute(TrackID),
    DeleteTrack(TrackID),
    ToggleSequencerWindow,
    OpenTrackFileLocation(String),

    // toolbar
    Stop,
    ChangeBpm(f32),
    TogglePlay,
    ProjectFileDialog,
    TrackFileDialog,

    // menus
    OpenTrackMenu(f32, f32, PatternID, TrackID),
    CloseContextMenu,

    // patterns
    //
    DeletePlaylistAudioBlock(AudioBlockID),
    DeletePattern(PatternID),
    DuplicatePattern(PatternID),
    CreatePattern,
    ClearPattern(PatternID),
    AddPlaylistAudioBlock(u32, u32, u32, AudioBlockType),
    OpenPatternMenu(PatternID, f32, f32),
    OpenSequencerForPattern(PatternID),
    StartResizeEvent(AudioBlockID),
    ChangePlaylistTool(PlaylistTool),

    // renaming
    StartRenamingPattern(PatternID),
    StartRenamingTrack(TrackID),

    // piano roll
    TogglePianoRollWindow,
    LoadPianoRoll(PianoRollState),

    // toggle ui components
    ToggleStretchMode,
    ToggleMixerWindow,
    TogglePlaylistWindow,
    ToggleTrackWindow(TrackID),
    TogglePatternTray,
    ToggleTrackTray,
    SelectPattern(PatternID),
    SelectTrackTray(TrackID),
    SelectAudioBlockToCopy(AudioBlockID),
    DuplicatePlaylistAudioBlock(AudioBlockID, u32, u32),

    // modal controls
    ModalConfirmSaveAndExit,
    ModalConfirmDiscardAndExit,
    ModalCancelExit,

    // file system
    FsToggleDir(PathBuf),
    FsPreviewSample(PathBuf),
    FsStartDragFile(PathBuf),
    FSEndDragFile(PathBuf, u32, usize), // track, step

    // no click result
    None,
}

impl App {
    pub(super) fn handle_click_result(
        &mut self,
        result: ClickResult,
        event_loop: &ActiveEventLoop,
    ) {
        let State::Ready(gfx) = &mut self.state else {
            return;
        };

        match result {
            ClickResult::OpenSequencerForPattern(pattern_id) => {
                gfx.active_pattern_id = pattern_id;
                if let Some(win) = gfx
                    .mini_windows
                    .iter_mut()
                    .find(|w| matches!(w.window_kind, WindowKind::Sequencer))
                {
                    bring_to_front(&mut gfx.z_order, SEQUENCER_ID);
                    win.is_open = true;
                }
            }
            // UI EDITS
            ClickResult::DuplicatePlaylistAudioBlock(block_id, track, step) => {
                if let Some(block) = gfx.audio_blocks.iter().find(|b| b.id == block_id).cloned() {
                    let duplicate = block.duplicate(
                        next_id(gfx.audio_blocks.iter().map(|b| b.id.0), AudioBlockID),
                        track,
                        step,
                    );
                    gfx.audio_blocks.push(duplicate.clone());
                    self.producer
                        .try_push(AudioCommand::CreateAudioBlock(duplicate))
                        .ok();
                    self.project_is_dirty = true;
                    self.project_revision += 1;
                }
            }

            ClickResult::ToggleStretchMode => {
                gfx.stretch_mode = !gfx.stretch_mode;
            }
            ClickResult::ModalCancelExit => {
                gfx.show_save_modal = false;
            }
            ClickResult::ChangePlaylistTool(tool) => gfx.playlist_tool = tool,
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
                if let Some(track) = gfx.tracks.iter().find(|t| t.data.id == id) {
                    gfx.renaming = Some(RenameState {
                        target: RenameTarget::Track(id),

                        edited_name: track.data.name.clone(),
                        cursor: track.data.name.len(),
                    });
                }
            }
            ClickResult::FsStartDragFile(path) => gfx.dragging_file = Some(path),
            ClickResult::SelectAudioBlockToCopy(block_id) => {
                gfx.playlist_action = PlaylistAction::PlaceCopy(block_id);
            }
            ClickResult::SelectTrackTray(id) => {
                eprintln!("set playlist_action: PlaceFresh(Sample({:?}))", id);
                gfx.playlist_action = PlaylistAction::PlaceFresh(AudioBlockType::Sample(id));
            }
            ClickResult::SelectPattern(pattern_id) => {
                gfx.active_pattern_id = pattern_id;
                gfx.playlist_action =
                    PlaylistAction::PlaceFresh(AudioBlockType::Pattern(pattern_id));
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
            ClickResult::StartResizeEvent(id) => gfx.resizing_audio_block = Some(id),

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
            ClickResult::OpenPatternMenu(pattern_id, x, y) => {
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

            ClickResult::ToggleTrackWindow(track_id) => {
                let Some(track) = gfx.tracks.iter().find(|t| t.data.id == track_id) else {
                    return; // track no longer exists; nothing to open
                };
                gfx.piano_roll_state = Some(super::PianoRollState {
                    pattern_id: gfx.active_pattern_id,
                    track_id,
                    scroll_offset: super::ScrollOffset {
                        x: 0.0,
                        y: PIANO_ROLL_DEFAULT_Y,
                    },
                });
                if let Some(pos) = gfx
                    .mini_windows
                    .iter()
                    .position(|w| w.window_kind == WindowKind::TrackDetail(track_id))
                {
                    gfx.mini_windows[pos].is_open = !gfx.mini_windows[pos].is_open;
                } else {
                    gfx.mini_windows.push(MiniWindow {
                        x: 128.0,
                        y: 128.0,
                        width: 600.0,
                        height: 500.0,
                        title: track.data.name.clone(),
                        is_open: true,
                        window_kind: WindowKind::TrackDetail(track_id),
                    });
                    let new_id = gfx.mini_windows.len() - 1;
                    gfx.z_order.push(new_id);
                }
            }
            ClickResult::CreatePattern => {
                let pattern = PatternData {
                    id: next_id(gfx.patterns.iter().map(|p| p.id.0), PatternID),
                    name: format!("Pattern {}", gfx.patterns.len() + 1),
                    sequences: gfx
                        .tracks
                        .iter()
                        .map(|track| Sequence {
                            track_id: track.data.id,
                            steps: vec![Note::DEFAULT; 16],
                        })
                        .collect(),
                };
                gfx.patterns.push(pattern.clone());
                self.producer
                    .try_push(AudioCommand::CreatePattern(pattern))
                    .ok();
                self.project_is_dirty = true;
                self.project_revision += 1;
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
                if let Some(pattern) = gfx.patterns.iter_mut().find(|p| p.id == pattern_id) {
                    for sequence in &mut pattern.sequences {
                        sequence.steps.fill(Note::DEFAULT);
                    }
                    self.project_is_dirty = true;
                    self.project_revision += 1;
                }
                self.producer
                    .try_push(AudioCommand::ClearPattern(pattern_id))
                    .ok();
                gfx.context_menu = None;
            }
            ClickResult::ModalConfirmSaveAndExit => {
                gfx.show_save_modal = false;
                self.is_closing = true;
                self.close_save_submitted = false;
            }
            ClickResult::ModalConfirmDiscardAndExit => {
                gfx.show_save_modal = false;
                crate::config::save(&self.config);
                self.pause_audio_stream();
                event_loop.exit();
            }
            // play a 5 second sample of the audio clip
            ClickResult::FsPreviewSample(track_path) => {
                let (preview, channels) =
                    crate::project::path_to_preview(&track_path.to_string_lossy(), 5);
                self.producer
                    .try_push(AudioCommand::PreviewSample(preview, channels))
                    .ok();
            }
            ClickResult::ChangeBpm(bpm) => {
                gfx.bpm = bpm;
                self.producer.try_push(AudioCommand::ChangeBpm(bpm)).ok();
                self.project_is_dirty = true;
                self.project_revision += 1;
            }
            ClickResult::DuplicatePattern(pattern_id) => {
                if let Some(pattern) = gfx.patterns.iter().find(|p| p.id == pattern_id).cloned() {
                    let duplicate =
                        pattern.duplicate(next_id(gfx.patterns.iter().map(|p| p.id.0), PatternID));
                    gfx.patterns.push(duplicate.clone());
                    self.producer
                        .try_push(AudioCommand::CreatePattern(duplicate))
                        .ok();
                    self.project_is_dirty = true;
                    self.project_revision += 1;
                }
                gfx.context_menu = None;
            }
            ClickResult::ToggleNote(pattern_id, track_id, step_idx, pitch) => {
                if let Some(pattern) = gfx.patterns.iter_mut().find(|p| p.id == pattern_id) {
                    if let Some(sequence) = pattern
                        .sequences
                        .iter_mut()
                        .find(|sequence| sequence.track_id == track_id)
                    {
                        if step_idx >= sequence.steps.len() {
                            sequence.steps.resize(step_idx + 1, Note::DEFAULT);
                        }
                        let note = &mut sequence.steps[step_idx];
                        *note = if note.velocity > 0.0 && note.pitch == pitch {
                            Note::DEFAULT
                        } else {
                            Note {
                                velocity: 95.0,
                                pitch,
                            }
                        };
                        while sequence
                            .steps
                            .last()
                            .is_some_and(|note| note.velocity == 0.0)
                        {
                            sequence.steps.pop();
                        }
                    } else {
                        let mut steps = vec![Note::DEFAULT; step_idx + 1];
                        steps[step_idx] = Note {
                            velocity: 95.0,
                            pitch,
                        };
                        pattern.sequences.push(Sequence { track_id, steps });
                    }
                    self.project_is_dirty = true;
                    self.project_revision += 1;
                }
                self.producer
                    .try_push(AudioCommand::ToggleNote(
                        pattern_id, track_id, step_idx, pitch,
                    ))
                    .ok();
            }
            ClickResult::DeletePlaylistAudioBlock(id) => {
                gfx.audio_blocks.retain(|block| block.id != id);
                gfx.waveform_cache.entries.remove(&id);
                self.producer
                    .try_push(AudioCommand::DeleteAudioBlock(id))
                    .ok();
                self.project_is_dirty = true;
                self.project_revision += 1;
            }
            ClickResult::AddPlaylistAudioBlock(track, start_step, length, block_type) => {
                let audio_block =
                    super::make_audio_block(gfx, track, start_step, length, block_type);
                gfx.audio_blocks.push(audio_block.clone());
                // request audio thread to create audio block
                self.producer
                    .try_push(AudioCommand::CreateAudioBlock(audio_block))
                    .ok();
                self.project_is_dirty = true;
                self.project_revision += 1;
            }
            ClickResult::DeletePattern(pattern_id) => {
                gfx.patterns.retain(|pattern| pattern.id != pattern_id);
                let removed_ids: Vec<_> = gfx
                    .audio_blocks
                    .iter()
                    .filter(|block| block.block_type == AudioBlockType::Pattern(pattern_id))
                    .map(|block| block.id)
                    .collect();
                gfx.audio_blocks
                    .retain(|block| block.block_type != AudioBlockType::Pattern(pattern_id));
                for id in removed_ids {
                    gfx.waveform_cache.entries.remove(&id);
                }
                gfx.context_menu = None;
                self.producer
                    .try_push(AudioCommand::DeletePattern(pattern_id))
                    .ok();
                self.project_is_dirty = true;
                self.project_revision += 1;
            }
            ClickResult::ToggleStep(pattern_id, track_id, step) => {
                if let Some(pattern) = gfx.patterns.iter_mut().find(|p| p.id == pattern_id) {
                    if let Some(sequence) = pattern
                        .sequences
                        .iter_mut()
                        .find(|sequence| sequence.track_id == track_id)
                    {
                        if step >= sequence.steps.len() {
                            sequence.steps.resize(step + 1, Note::DEFAULT);
                        }
                        sequence.steps[step] = if sequence.steps[step].velocity > 0.0 {
                            Note::DEFAULT
                        } else {
                            Note {
                                velocity: 95.0,
                                pitch: 60,
                            }
                        };
                        while sequence
                            .steps
                            .last()
                            .is_some_and(|note| note.velocity == 0.0)
                        {
                            sequence.steps.pop();
                        }
                    } else {
                        let mut sequence = Sequence {
                            track_id,
                            steps: vec![Note::DEFAULT; step + 1],
                        };
                        sequence.steps[step] = Note {
                            velocity: 95.0,
                            pitch: 60,
                        };
                        pattern.sequences.push(sequence);
                    }
                    self.project_is_dirty = true;
                    self.project_revision += 1;
                }
                self.producer
                    .try_push(AudioCommand::ToggleStep(pattern_id, track_id, step))
                    .ok();
            }
            ClickResult::Stop => {
                gfx.is_playing = false;
                gfx.active_step = 0;
                self.producer.try_push(AudioCommand::Stop).ok();
            }
            ClickResult::ToggleTrackMute(track_id) => {
                // The sequencer interaction already changed the app-owned track
                // value while producing this click result. Only mirror the edit
                // to audio here; toggling gfx again would undo the UI mutation.
                self.project_is_dirty = true;
                self.project_revision += 1;
                self.producer
                    .try_push(AudioCommand::ToggleTrackMute(track_id))
                    .ok();
            }
            ClickResult::TogglePlay => {
                // Reflect the requested state immediately. The audio status
                // acknowledgement can be dropped if its bounded UI ring is full.
                gfx.is_playing = !gfx.is_playing;
                self.producer.try_push(AudioCommand::TogglePlay).ok();
            }
            ClickResult::DeleteTrack(track_id) => {
                if let Some(position) = gfx.tracks.iter().position(|t| t.data.id == track_id) {
                    gfx.tracks.remove(position);
                    for pattern in &mut gfx.patterns {
                        pattern
                            .sequences
                            .retain(|sequence| sequence.track_id != track_id);
                    }
                    let removed_ids: Vec<_> = gfx
                        .audio_blocks
                        .iter()
                        .filter(|block| block.block_type == AudioBlockType::Sample(track_id))
                        .map(|block| block.id)
                        .collect();
                    gfx.audio_blocks
                        .retain(|block| block.block_type != AudioBlockType::Sample(track_id));
                    for id in removed_ids {
                        gfx.waveform_cache.entries.remove(&id);
                    }
                    gfx.context_menu = None;
                    gfx.mini_windows[SEQUENCER_ID].height = 100.0
                        + crate::graphics::mini_window::sequencer::TRACK_GAP
                            * gfx.tracks.len() as f32;
                    self.project_is_dirty = true;
                    self.project_revision += 1;
                }
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
