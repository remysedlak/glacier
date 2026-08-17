//! handle all commands incoming from audio thread
use super::{App, State};
use crate::graphics::{
    bring_to_front,
    mini_window::{sequencer::TRACK_GAP, SEQUENCER_ID},
};
use crate::project::{AudioBlock, AudioBlockType, PatternData, Track};
use cpal::traits::StreamTrait;

/// UiCommands are used to sync the audio engine to the graphics engine
pub enum UiCommand {
    TrackLevel(u32, f32, f32, f32),
    TrackLoaded(Track),
    TrackRenamed(u32, String),
    TrackDeleted(u32),

    PatternRenamed(usize, String),
    PatternLoaded(PatternData),
    PatternDeleted(usize),

    AudioBlockDeleted(usize),
    AudioBlockLoaded(AudioBlock),

    MasterLevel(f32, f32, f32),
    SampleRateLoaded(f32),
    StepAdvanced(usize),
    PlayheadPosition(f32),
    SpectrumFrame(Vec<f32>),

    ShutdownComplete,
    SaveComplete,

    LoadProject {
        tracks: Vec<Track>,
        patterns: Vec<PatternData>,
        events: Vec<AudioBlock>,
        bpm: f32,
        master_volume: f32,
        project_path: String,
    },
}

impl App {
    pub(super) fn handle_ui_command(&mut self, cmd: UiCommand, should_exit: &mut bool) {
        use crate::audio::AudioCommand;
        use ringbuf::traits::{Producer, Split};
        use ringbuf::HeapRb;

        let App {
            state,
            producer,
            consumer,
            pending_drop,
            project_is_dirty,
            config,
            stream,
            pending_project,
            ..
        } = self;

        let State::Ready(gfx) = state else { return };

        match cmd {
            UiCommand::AudioBlockLoaded(audio_block) => {
                gfx.events.push(audio_block);
                self.project_is_dirty = true;
            }
            UiCommand::TrackDeleted(track_id) => {
                if let Some(pos) = gfx.tracks.iter().position(|t| t.data.id == track_id) {
                    gfx.tracks.remove(pos);
                }
                gfx.mini_windows[SEQUENCER_ID].height = 100.0 + TRACK_GAP * gfx.tracks.len() as f32;
                gfx.events.retain(|e| {
                    if let AudioBlockType::Sample(id) = e.block_type {
                        id != track_id as usize
                    } else {
                        true
                    }
                });
                gfx.context_menu = None;
                self.project_is_dirty = true;
            }
            UiCommand::AudioBlockDeleted(event_id) => {
                gfx.events.retain(|e| e.id != event_id);
                self.project_is_dirty = true;
            }
            UiCommand::PatternDeleted(pattern_id) => {
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
            UiCommand::SpectrumFrame(samples) => gfx.spectrum = samples,
            UiCommand::LoadProject {
                tracks,
                patterns,
                events,
                bpm,
                master_volume,
                project_path,
            } => {
                gfx.tracks = tracks;
                gfx.patterns = patterns;
                gfx.events = events;
                gfx.bpm = bpm;
                gfx.master_volume = master_volume;
                gfx.project_path = project_path;
                gfx.mini_windows[SEQUENCER_ID].height = 100.0 + TRACK_GAP * gfx.tracks.len() as f32;
            }
            UiCommand::PatternRenamed(id, name) => {
                if let Some(pattern) = gfx.patterns.iter_mut().find(|p| p.id == id) {
                    pattern.name = name;
                }
            }
            UiCommand::TrackRenamed(id, name) => {
                if let Some(track) = gfx.tracks.iter_mut().find(|t| t.data.id == id) {
                    track.data.name = name;
                }
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
            UiCommand::TrackLoaded(track) => {
                let track_id = track.data.id as usize;
                gfx.active_tray = AudioBlockType::Sample(track_id);
                gfx.tracks.push(track);
                let win = &mut gfx.mini_windows[SEQUENCER_ID];
                win.height = 100.0 + TRACK_GAP * gfx.tracks.len() as f32;

                if let Some((playlist_track, step)) = pending_drop.take() {
                    producer
                        .try_push(AudioCommand::CreateAudioBlock(
                            playlist_track,
                            step as u32,
                            1,
                            AudioBlockType::Sample(track_id),
                        ))
                        .ok();
                } else {
                    win.is_open = true;
                    bring_to_front(&mut gfx.z_order, SEQUENCER_ID);
                }
            }
            UiCommand::SampleRateLoaded(rate) => gfx.sample_rate = rate,
            UiCommand::PatternLoaded(pattern) => gfx.load_pattern(pattern),
            UiCommand::StepAdvanced(step) => {
                gfx.active_step = step;
                gfx.request_redraw();
            }
            UiCommand::ShutdownComplete => {
                crate::config::save(config);
                let _ = stream.pause();
                *should_exit = true;
            }
            UiCommand::SaveComplete => {
                if let Some(path) = pending_project.take() {
                    let _ = stream.pause();
                    let (audio_prod, audio_cons) = HeapRb::<AudioCommand>::new(64).split();
                    let (ui_prod, ui_cons) = HeapRb::<UiCommand>::new(64).split();
                    *producer = audio_prod;
                    *consumer = ui_cons;
                    *stream = crate::audio::init(audio_cons, ui_prod, Some(path));
                }
                *project_is_dirty = false;
            }
        }
    }
}
