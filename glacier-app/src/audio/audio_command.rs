use crate::{
    app::UiCommand,
    audio::EngineState,
    project::{
        AudioBlock, AudioBlockID, AudioBlockType, Note, PatternData, PatternID, Sequence, Track,
        TrackID,
    },
};
use ringbuf::traits::Producer;

/// A command sent from the UI thread to control the audio engine
pub enum AudioCommand {
    // composition details
    ToggleStep(PatternID, TrackID, usize), // pattern_id, track_id, step_idx
    ToggleNote(PatternID, TrackID, usize, u8), // pattern_id, track_id, step_idx, pitch
    ChangeBpm(f32),

    // audio blocks
    DeleteAudioBlock(AudioBlockID),
    CreateAudioBlock(AudioBlock),
    ResizeAudioBlock(AudioBlockID, u32),

    // mixing
    ChangeMasterVolume(f32),
    ToggleTrackMute(TrackID),
    ChangeTrackVolume(TrackID, f32),

    // control
    TogglePlay,
    Stop,
    PreviewSample(Vec<f32>, u16),

    // patterns
    CreatePattern(PatternData),
    DeletePattern(PatternID),
    ClearPattern(PatternID),

    // renaming state
    RenamePattern(PatternID, String),
    RenameTrack(TrackID, String),

    // tracks
    LoadTrack(Track),
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
            }
        }
        AudioCommand::RenamePattern(pattern_id, name) => {
            if let Some(pattern) = engine_state
                .patterns
                .iter_mut()
                .find(|p| p.id == pattern_id)
            {
                pattern.name = name.clone();
            }
        }
        AudioCommand::RenameTrack(track_id, name) => {
            if let Some(track) = engine_state
                .tracks
                .iter_mut()
                .find(|t| t.data.id == track_id)
            {
                track.data.name = name.clone();
            }
        }
        AudioCommand::PreviewSample(samples, channels) => {
            engine_state.preview_samples = samples;
            engine_state.preview_channels = channels;
            engine_state.preview_position = 0.0;
        }
        AudioCommand::ResizeAudioBlock(audio_block_id, new_length) => {
            if let Some(audio_block) = engine_state
                .audio_blocks
                .iter_mut()
                .find(|audio_block| audio_block.id == audio_block_id)
            {
                audio_block.length = new_length;
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
            }
        }
        AudioCommand::CreatePattern(pattern) => engine_state.patterns.push(pattern),
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
        }
        AudioCommand::DeleteAudioBlock(audio_block_id) => {
            engine_state.audio_blocks.retain(|e| e.id != audio_block_id);
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
        AudioCommand::CreateAudioBlock(block) => engine_state.audio_blocks.push(block),
        AudioCommand::ChangeMasterVolume(new_volume) => engine_state.master_volume = new_volume,
        AudioCommand::ChangeTrackVolume(track_id, new_volume) => {
            if let Some(track) = engine_state
                .tracks
                .iter_mut()
                .find(|t| t.data.id == track_id)
            {
                track.data.track_volume = new_volume;
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
            };
        }

        AudioCommand::LoadTrack(track) => {
            engine_state.tracks.push(track);
        }
        AudioCommand::ChangeBpm(new_bpm) => {
            engine_state.bpm = new_bpm;
        }
        AudioCommand::ToggleTrackMute(track_id) => {
            if let Some(track) = engine_state
                .tracks
                .iter_mut()
                .find(|t| t.data.id == track_id)
            {
                track.mute();
            }
        }
        AudioCommand::TogglePlay => {
            engine_state.is_playing = !engine_state.is_playing;
            engine_state
                .audio_producer
                .try_push(UiCommand::PlayToggled(engine_state.is_playing))
                .ok();
        }
    }
}
