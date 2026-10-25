//! handle all commands incoming from audio thread
use super::{App, State};
use crate::project::{ProjectSaveResult, SavePurpose, TrackID};
use winit::event_loop::ActiveEventLoop;

/// UiCommands are used to sync the audio engine to the graphics engine
pub enum UiCommand {
    TrackLevel(TrackID, f32, f32, f32),

    MasterLevel(f32, f32, f32),
    StepAdvanced(usize),
    PlayheadPosition(f32),
    PlaybackStopped,
    PlayToggled(bool),
}

impl App {
    pub(super) fn handle_ui_command(&mut self, cmd: UiCommand) {
        let State::Ready(gfx) = &mut self.state else {
            return;
        };

        match cmd {
            UiCommand::PlayToggled(is_playing) => gfx.is_playing = is_playing,
            UiCommand::PlaybackStopped => {
                gfx.is_playing = false;
                gfx.active_step = 0;
            }
            UiCommand::PlayheadPosition(beat) => gfx.playhead_beat = beat,
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
            UiCommand::StepAdvanced(step) => {
                gfx.active_step = step;
                gfx.request_redraw();
            }
        }
    }

    pub(super) fn handle_save_result(
        &mut self,
        result: ProjectSaveResult,
        event_loop: &ActiveEventLoop,
    ) {
        let close_after_regular_save = self.is_closing
            && result.purpose == SavePurpose::Regular
            && result.revision == self.project_revision;
        if result.purpose == SavePurpose::Regular {
            self.regular_saves_in_flight
                .remove(&(result.path.clone(), result.revision));
        }

        if let Err(error) = &result.result {
            log::error!("failed to save project {}: {error}", result.path);
        } else {
            println!("saved to {}", result.path);
        }

        let purpose = if close_after_regular_save {
            SavePurpose::Close
        } else {
            result.purpose
        };
        match purpose {
            SavePurpose::Regular => {
                if result.result.is_ok() {
                    if let State::Ready(gfx) = &mut self.state {
                        gfx.project_path = result.path.clone();
                    }
                    if result.revision == self.project_revision {
                        self.project_is_dirty = false;
                        if let Some(path) = self.pending_project.take() {
                            self.project_load_rx =
                                Some(crate::project::spawn_project_load(path, false));
                        }
                    } else if self.pending_project.is_some() {
                        let State::Ready(gfx) = &self.state else {
                            return;
                        };
                        super::submit_project_save(
                            &self.save_worker,
                            &mut self.regular_saves_in_flight,
                            &self.project_name,
                            gfx,
                            SavePurpose::ProjectSwitch,
                            self.project_revision,
                            None,
                        );
                    }
                } else if self.pending_project.is_some() {
                    self.pending_project = None;
                }
            }
            SavePurpose::ProjectSwitch => {
                if result.result.is_ok() && result.revision == self.project_revision {
                    self.project_is_dirty = false;
                    if let Some(path) = self.pending_project.take() {
                        self.project_load_rx =
                            Some(crate::project::spawn_project_load(path, false));
                    }
                } else if result.result.is_ok() {
                    // The project changed while the snapshot was being written; save the
                    // latest state before starting the requested project load.
                    if self.pending_project.is_some() {
                        let State::Ready(gfx) = &self.state else {
                            return;
                        };
                        super::submit_project_save(
                            &self.save_worker,
                            &mut self.regular_saves_in_flight,
                            &self.project_name,
                            gfx,
                            SavePurpose::ProjectSwitch,
                            self.project_revision,
                            None,
                        );
                    }
                } else {
                    // Keep the current project and its unsaved changes intact.
                    self.pending_project = None;
                }
            }
            SavePurpose::Close => {
                if result.result.is_ok() && result.revision == self.project_revision {
                    self.project_is_dirty = false;
                    crate::config::save(&self.config);
                    self.pause_audio_stream();
                    event_loop.exit();
                } else if result.result.is_ok() {
                    if let State::Ready(gfx) = &self.state {
                        super::submit_project_save(
                            &self.save_worker,
                            &mut self.regular_saves_in_flight,
                            &self.project_name,
                            gfx,
                            SavePurpose::Close,
                            self.project_revision,
                            None,
                        );
                    }
                } else {
                    self.is_closing = false;
                    self.close_save_submitted = false;
                    if let State::Ready(gfx) = &mut self.state {
                        gfx.show_save_modal = true;
                        gfx.request_redraw();
                    }
                }
            }
        }
    }
}
