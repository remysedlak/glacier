use crate::{
    app::UiCommand,
    audio::EngineState,
    project::{
        AudioBlock, AudioBlockID, AudioBlockType, Note, PatternData, PatternID, Project, Sequence,
        Track, TrackData, TrackID,
    },
};

use ringbuf::traits::Producer;

/// A command sent from the UI thread to control the audio engine
pub enum AudioCommand {
    // composition details
    ToggleStep(PatternID, TrackID, usize), // pattern_id, track_id, step_idx
    ToggleNote(PatternID, TrackID, usize, u8), // pattern_id, track_id, step_idx, pitch
    ChangeBpm(f32),
    DeleteAudioBlock(AudioBlockID),
    CreateAudioBlock(TrackID, u32, u32, AudioBlockType), // track_id, start_step, length, block_type
    ResizeAudioBlock(AudioBlockID, u32),

    // mixing
    ChangeMasterVolume(f32),
    ToggleTrackMute(TrackID),
    ChangeTrackVolume(TrackID, f32),

    // control
    TogglePlay,
    Stop,
    PreviewSample(Vec<f32>),

    // project state
    Shutdown,
    ShutdownWithoutSaving,
    SaveProject,
    SetProjectPath(String),

    // patterns
    DuplicatePattern(PatternID),
    CreatePattern,
    DeletePattern(PatternID),
    ClearPattern(PatternID),

    // renaming state
    RenamePattern(PatternID, String),
    RenameTrack(TrackID, String),

    // tracks
    LoadTrack(TrackData, Vec<f32>),
    DeleteTrack(TrackID),
}

// patterns, producer, audio_blocks
pub fn handle_command(cmd: AudioCommand, engine_state: &mut EngineState) {
    match cmd {
        // clear a patterns step data by looping through each sequencer and emptying the note
        AudioCommand::ClearPattern(pattern_id) => {
            if let Some(pattern) = engine_state
                .patterns
                .iter_mut()
                .find(|p| p.id == pattern_id)
            {
                for seq in pattern.sequences.iter_mut() {
                    for note in seq.steps.iter_mut() {
                        *note = Note::DEFAULT;
                    }
                }
                // update the ui
                engine_state
                    .audio_producer
                    .try_push(UiCommand::PatternUpdated(pattern.clone()))
                    .ok();
            }
        }
        AudioCommand::ShutdownWithoutSaving => {
            if !engine_state.is_playing {
                engine_state
                    .audio_producer
                    .try_push(UiCommand::ShutdownComplete)
                    .ok();
            }
            engine_state.is_shutting_down = true;
        }
        AudioCommand::RenamePattern(pattern_id, name) => {
            if let Some(pattern) = engine_state
                .patterns
                .iter_mut()
                .find(|p| p.id == pattern_id)
            {
                pattern.name = name.clone();
                engine_state
                    .audio_producer
                    .try_push(UiCommand::PatternUpdated(pattern.clone()))
                    .ok();
            }
        }
        AudioCommand::RenameTrack(track_id, name) => {
            if let Some(track) = engine_state
                .tracks
                .iter_mut()
                .find(|t| t.data.id == track_id)
            {
                track.data.name = name.clone();
                engine_state
                    .audio_producer
                    .try_push(UiCommand::TrackUpdated(track.data.clone()))
                    .ok();
            }
        }
        AudioCommand::PreviewSample(samples) => {
            engine_state.preview_samples = samples;
            engine_state.preview_position = 0.0;
        }
        AudioCommand::SetProjectPath(new_path) => engine_state.project_path = new_path,
        AudioCommand::ResizeAudioBlock(audio_block_id, new_length) => {
            if let Some(audio_block) = engine_state
                .audio_blocks
                .iter_mut()
                .find(|audio_block| audio_block.id == audio_block_id)
            {
                audio_block.length = new_length;
            }
        }
        AudioCommand::DuplicatePattern(pattern_id) => {
            if let Some(pattern) = engine_state
                .patterns
                .iter()
                .find(|p| p.id == pattern_id)
                .cloned()
            {
                let new_pattern = pattern.duplicate(
                    engine_state
                        .patterns
                        .iter()
                        .map(|x| x.id.0)
                        .max()
                        .map(|m| PatternID(m + 1))
                        .unwrap_or(PatternID(0)),
                );
                engine_state.patterns.push(new_pattern.clone());
                // update the ui
                engine_state
                    .audio_producer
                    .try_push(UiCommand::PatternUpdated(new_pattern))
                    .ok();
            }
        }

        // activate a step within a track within a pattern
        AudioCommand::ToggleNote(pattern_id, track_id, step_idx, pitch) => {
            if let Some(pattern) = engine_state
                .patterns
                .iter_mut()
                .find(|p| p.id == pattern_id)
            {
                if let Some(seq) = pattern
                    .sequences
                    .iter_mut()
                    .find(|s| s.track_id == track_id)
                {
                    if step_idx >= seq.steps.len() {
                        seq.steps.resize(step_idx + 1, Note::DEFAULT);
                    }
                    let note = &mut seq.steps[step_idx];
                    if note.velocity > 0.0 && note.pitch == pitch {
                        *note = Note::DEFAULT;
                    } else {
                        *note = Note {
                            velocity: 95.0,
                            pitch,
                        };
                    }
                    while seq.steps.last().map(|n| n.velocity == 0.0).unwrap_or(false) {
                        seq.steps.pop();
                    }
                } else {
                    let mut steps = vec![Note::DEFAULT; step_idx + 1];
                    steps[step_idx] = Note {
                        velocity: 95.0,
                        pitch,
                    };
                    pattern.sequences.push(Sequence { track_id, steps });
                }
                engine_state
                    .audio_producer
                    .try_push(UiCommand::PatternUpdated(pattern.clone()))
                    .ok();
            }
        }
        AudioCommand::CreatePattern => {
            let new_pattern_id = engine_state
                .patterns
                .iter()
                .map(|x| x.id.0)
                .max()
                .map(|m| PatternID(m + 1))
                .unwrap_or(PatternID(0));

            // create name to be Nth pattern available
            let name = format!("Pattern {}", engine_state.patterns.len() + 1);

            let sequences = engine_state
                .tracks
                .iter()
                .map(|instr| Sequence {
                    track_id: instr.data.id,
                    steps: vec![Note::DEFAULT; 16],
                })
                .collect();
            let p = PatternData {
                id: new_pattern_id,
                name,
                sequences,
            };
            engine_state.patterns.push(p.clone());
            engine_state
                .audio_producer
                .try_push(UiCommand::PatternUpdated(p))
                .ok();
        }
        AudioCommand::DeletePattern(pattern_id) => {
            // remove the pattern from list of patterns
            engine_state.patterns.retain(|p| p.id != pattern_id);
            // remove the pattern from list of audio_blocks
            engine_state.audio_blocks.retain(|e| {
                if let AudioBlockType::Pattern(pid) = e.block_type {
                    pid != pattern_id
                } else {
                    true
                }
            });
            engine_state
                .audio_producer
                .try_push(UiCommand::PatternDeleted(pattern_id))
                .ok();
        }
        AudioCommand::DeleteAudioBlock(audio_block_id) => {
            engine_state.audio_blocks.retain(|e| e.id != audio_block_id);
            engine_state
                .audio_producer
                .try_push(UiCommand::AudioBlockDeleted(audio_block_id))
                .ok();
        }
        AudioCommand::Stop => {
            engine_state.is_playing = false;
            engine_state.current_step = 0;
            engine_state
                .audio_producer
                .try_push(UiCommand::PlayheadPosition(0.0))
                .ok();
            engine_state
                .audio_producer
                .try_push(UiCommand::PlaybackStopped)
                .ok();
        }
        AudioCommand::CreateAudioBlock(track_id, start_step, length, block_type) => {
            let computed_length = match &block_type {
                AudioBlockType::Sample(sample_track_id) => engine_state
                    .tracks
                    .iter()
                    .find(|t| t.data.id == *sample_track_id)
                    .map(|track| {
                        let samples_per_step = glacier_dsp::samples_per_step(
                            engine_state.sample_rate,
                            engine_state.bpm,
                        );
                        let frames = (track.samples.len() / track.data.channels as usize) as f32;
                        (frames / samples_per_step).ceil() as u32
                    })
                    .unwrap_or(length),
                _ => length, // Pattern, Mixing — use whatever the UI sent
            };

            let audio_block = AudioBlock {
                id: engine_state
                    .audio_blocks
                    .iter()
                    .map(|x| x.id.0)
                    .max()
                    .map(|m| AudioBlockID(m + 1))
                    .unwrap_or(AudioBlockID(0)),
                track_id,
                start_step,
                length: computed_length,
                block_type,
                is_muted: false,
            };
            engine_state.audio_blocks.push(audio_block.clone());
            engine_state
                .audio_producer
                .try_push(UiCommand::AudioBlockLoaded(audio_block))
                .ok();
        }
        AudioCommand::ChangeMasterVolume(new_volume) => engine_state.master_volume = new_volume,
        AudioCommand::ChangeTrackVolume(track_id, new_volume) => {
            if let Some(track) = engine_state
                .tracks
                .iter_mut()
                .find(|t| t.data.id == track_id)
            {
                track.data.track_volume = new_volume;
                engine_state
                    .audio_producer
                    .try_push(UiCommand::TrackUpdated(track.data.clone()))
                    .ok();
            }
        }
        AudioCommand::ToggleStep(pattern_id, track_id, step_idx) => {
            if let Some(pattern) = engine_state
                .patterns
                .iter_mut()
                .find(|p| p.id == pattern_id)
            {
                if let Some(seq) = pattern
                    .sequences
                    .iter_mut()
                    .find(|s| s.track_id == track_id)
                {
                    if step_idx >= seq.steps.len() {
                        seq.steps.resize(step_idx + 1, Note::DEFAULT);
                    }
                    seq.steps[step_idx] = if seq.steps[step_idx].velocity > 0.0 {
                        Note::DEFAULT
                    } else {
                        Note {
                            velocity: 95.0,
                            pitch: 60,
                        }
                    };
                    while seq.steps.last().map(|n| n.velocity == 0.0).unwrap_or(false) {
                        seq.steps.pop();
                    }
                } else {
                    let mut seq = Sequence {
                        track_id,
                        steps: vec![Note::DEFAULT; step_idx + 1],
                    };
                    seq.steps[step_idx] = Note {
                        velocity: 95.0,
                        pitch: 60,
                    };
                    pattern.sequences.push(seq);
                }
                engine_state
                    .audio_producer
                    .try_push(UiCommand::PatternUpdated(pattern.clone()))
                    .ok();
            }
        }
        AudioCommand::DeleteTrack(track_id) => {
            if let Some(pos) = engine_state
                .tracks
                .iter()
                .position(|t| t.data.id == track_id)
            {
                let data_id = engine_state.tracks[pos].data.id;
                for pattern in engine_state.patterns.iter_mut() {
                    pattern.sequences.retain(|s| s.track_id != data_id);
                }
                engine_state.audio_blocks.retain(|e| {
                    if let crate::project::AudioBlockType::Sample(pid) = e.block_type {
                        pid != data_id
                    } else {
                        true
                    }
                });
                engine_state.tracks.remove(pos);
                engine_state
                    .audio_producer
                    .try_push(UiCommand::TrackDeleted(data_id))
                    .ok();
            };
        }

        AudioCommand::LoadTrack(mut track_data, samples) => {
            // used to be tracks.len()
            track_data.id = engine_state
                .tracks
                .iter()
                .map(|x| x.data.id.0)
                .max()
                .map(|m| TrackID(m + 1))
                .unwrap_or(TrackID(0));
            let track = Track::from_data(track_data, samples);
            engine_state.tracks.push(track.clone()); // ownership clone
            engine_state
                .audio_producer
                .try_push(UiCommand::TrackLoaded(track))
                .ok();
        }
        AudioCommand::ChangeBpm(new_bpm) => {
            engine_state.bpm = new_bpm;
            engine_state
                .audio_producer
                .try_push(UiCommand::BpmChanged(new_bpm))
                .ok();
        }
        AudioCommand::ToggleTrackMute(track_id) => {
            if let Some(track) = engine_state
                .tracks
                .iter_mut()
                .find(|t| t.data.id == track_id)
            {
                track.mute();
                engine_state
                    .audio_producer
                    .try_push(UiCommand::TrackUpdated(track.data.clone()))
                    .ok();
            }
        }
        AudioCommand::TogglePlay => {
            engine_state.is_playing = !engine_state.is_playing;
            engine_state
                .audio_producer
                .try_push(UiCommand::PlayToggled)
                .ok();
        }
        AudioCommand::SaveProject => {
            let project = Project::new(
                engine_state.project_name.clone(),
                engine_state.bpm,
                engine_state.master_volume,
                &engine_state.tracks,
                engine_state.patterns.clone(),
                engine_state.audio_blocks.clone(),
            );
            project.save_to_toml(&engine_state.project_path);
            engine_state
                .audio_producer
                .try_push(UiCommand::SaveComplete)
                .ok();
            println!("saved to {}", &engine_state.project_path);
        }
        AudioCommand::Shutdown => {
            let project = Project::new(
                engine_state.project_name.clone(),
                engine_state.bpm,
                engine_state.master_volume,
                &engine_state.tracks,
                engine_state.patterns.clone(),
                engine_state.audio_blocks.clone(),
            );
            project.save_to_toml(&engine_state.project_path);

            // save is complete
            engine_state
                .audio_producer
                .try_push(UiCommand::SaveComplete)
                .ok();
            if !engine_state.is_playing {
                engine_state
                    .audio_producer
                    .try_push(UiCommand::ShutdownComplete)
                    .ok();
            }
            engine_state.is_shutting_down = true;
        }
    }
}
