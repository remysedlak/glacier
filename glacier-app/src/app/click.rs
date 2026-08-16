//! handle all mouse click interactivity
use super::{App, State};
use crate::audio::AudioCommand;
use crate::graphics::{
    context_menu::{ContextMenu, ContextMenuKind},
    mini_window::{
        piano_roll::PIANO_ROLL_DEFAULT_Y, sequencer::TRACK_GAP, MiniWindow, WindowKind, MIXER_ID,
        PIANO_ROLL_ID, PLAYLIST_ID, SEQUENCER_ID,
    },
    primitives::{RenameState, RenameTarget},
    {bring_to_front, ClickResult},
};
use crate::project::{AudioBlock, AudioBlockType};
use rfd::FileDialog;
use ringbuf::traits::Producer;
use std::path::PathBuf;
use std::thread;
use winit::event_loop::ActiveEventLoop;

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
            ClickResult::ClearPattern(pattern_id) => {
                self.producer
                    .try_push(AudioCommand::ClearPattern(pattern_id))
                    .ok();
                gfx.context_menu = None;
                self.project_is_dirty = true;
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
            ClickResult::ModalCancelExit => {
                gfx.show_save_modal = false;
            }
            ClickResult::StartRenamingPattern(id) => {
                gfx.context_menu = None;
                if let Some(pattern) = gfx.patterns.iter().find(|p| p.id == id) {
                    gfx.renaming = Some(RenameState {
                        target: RenameTarget::Pattern(id),
                        original_name: pattern.name.clone(),
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
                        original_name: track.data.name.clone(),
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
            ClickResult::FsPreviewSample(track_path) => {
                let preview = crate::project::path_to_preview(&track_path.to_string_lossy(), 5);
                self.producer
                    .try_push(AudioCommand::PreviewSample(preview))
                    .ok();
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
                showfile::show_path_in_file_manager(path);
            }
            ClickResult::StartResizeEvent(id) => gfx.resizing_event = Some(id),
            ClickResult::DuplicatePattern(pattern_id) => {
                self.producer
                    .try_push(AudioCommand::DuplicatePattern(pattern_id))
                    .ok();
                self.project_is_dirty = true;
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
                if let Some(pattern) = gfx.patterns.iter_mut().find(|p| p.id == pattern_id) {
                    if let Some(seq) = pattern
                        .sequences
                        .iter_mut()
                        .find(|s| s.track_id == track_id)
                    {
                        if step_idx >= seq.steps.len() {
                            seq.steps
                                .resize(step_idx + 1, crate::project::Note::DEFAULT);
                        }
                        let note = &mut seq.steps[step_idx];
                        if note.velocity > 0.0 && note.pitch == pitch {
                            *note = crate::project::Note::DEFAULT;
                        } else {
                            *note = crate::project::Note {
                                velocity: 95.0,
                                pitch,
                            };
                        }
                        while seq.steps.last().map(|n| n.velocity == 0.0).unwrap_or(false) {
                            seq.steps.pop();
                        }
                    } else {
                        let mut steps = vec![crate::project::Note::DEFAULT; step_idx + 1];
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
                self.project_is_dirty = true;
            }
            ClickResult::DeletePlaylistPattern(id) => {
                gfx.events.retain(|e| e.id != id);
                self.producer
                    .try_push(AudioCommand::DeleteAudioBlock(id))
                    .ok();
                self.project_is_dirty = true;
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
                gfx.patterns.retain(|p| p.id != pattern_id);
                gfx.events.retain(|e| {
                    if let crate::project::AudioBlockType::Pattern(pid) = e.block_type {
                        pid != pattern_id
                    } else {
                        true
                    }
                });
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
                gfx.is_playing = false;
                gfx.active_step = 0;
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
                let data_id = gfx.tracks[track_id].data.id as usize;
                self.producer
                    .try_push(AudioCommand::DeleteTrack(track_id))
                    .ok();
                gfx.tracks.remove(track_id);
                gfx.mini_windows[SEQUENCER_ID].height = 100.0 + TRACK_GAP * gfx.tracks.len() as f32;
                gfx.events.retain(|e| {
                    if let AudioBlockType::Sample(id) = e.block_type {
                        id != data_id
                    } else {
                        true
                    }
                });
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
