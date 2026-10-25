use crate::{
    app::UiCommand,
    audio::{AudioProjectUpdate, EngineState, RetiredAudioData},
    project::{AudioBlockID, TrackID},
};
use ringbuf::traits::Producer;

/// Commands are either small scalar controls or an app-prepared ownership swap.
/// Project structure is never grown or destroyed by the audio callback.
pub enum AudioCommand {
    ReplaceProjectParts(Box<AudioProjectUpdate>),
    ChangeBpm(f32),
    ResizeAudioBlock(AudioBlockID, u32),
    ChangeMasterVolume(f32),
    SetTrackMute(TrackID, bool),
    ChangeTrackVolume(TrackID, f32),
    SetPlaying {
        is_playing: bool,
        reset_position: bool,
    },
    PreviewSample(std::sync::Arc<[f32]>, u16),
}

/// Apply one command. Heap-owning replacements are moved into `retired` and
/// returned to the app thread for destruction.
pub fn handle_command(
    cmd: AudioCommand,
    engine_state: &mut EngineState,
) -> Option<RetiredAudioData> {
    match cmd {
        AudioCommand::ReplaceProjectParts(mut update) => {
            // Keep active voices for tracks that still exist in the new project.
            // New snapshots are built on the app thread; swapping these fixed
            // runtime fields cannot allocate or free memory.
            if let Some(new_tracks) = update.tracks.as_mut() {
                let mut old_index = 0;
                for new_track in new_tracks {
                    while old_index < engine_state.tracks.len()
                        && engine_state.tracks[old_index].id < new_track.id
                    {
                        old_index += 1;
                    }
                    if let Some(old_track) = engine_state.tracks.get_mut(old_index) {
                        if old_track.id != new_track.id {
                            continue;
                        }
                        new_track.is_muted = old_track.is_muted;
                        new_track.track_volume = old_track.track_volume;
                        std::mem::swap(&mut new_track.voice_pool, &mut old_track.voice_pool);
                        std::mem::swap(&mut new_track.rms_l, &mut old_track.rms_l);
                        std::mem::swap(&mut new_track.rms_r, &mut old_track.rms_r);
                        std::mem::swap(&mut new_track.peak_hold, &mut old_track.peak_hold);
                        old_index += 1;
                    }
                }
            }

            if let Some(patterns) = update.patterns.as_mut() {
                std::mem::swap(&mut engine_state.patterns, patterns);
            }
            if let Some(tracks) = update.tracks.as_mut() {
                std::mem::swap(&mut engine_state.tracks, tracks);
            }
            if let Some(audio_blocks) = update.audio_blocks.as_mut() {
                std::mem::swap(&mut engine_state.audio_blocks, audio_blocks);
            }
            Some(RetiredAudioData::ProjectUpdate(update))
        }
        AudioCommand::PreviewSample(samples, channels) => {
            let old_samples = std::mem::replace(&mut engine_state.preview_samples, samples);
            engine_state.preview_channels = channels;
            engine_state.preview_position = 0.0;
            Some(RetiredAudioData::PreviewSamples(old_samples))
        }
        AudioCommand::ResizeAudioBlock(audio_block_id, new_length) => {
            if let Some(audio_block) = engine_state
                .audio_blocks
                .iter_mut()
                .find(|audio_block| audio_block.id == audio_block_id)
            {
                audio_block.length = new_length;
            }
            None
        }
        AudioCommand::ChangeMasterVolume(new_volume) => {
            engine_state.master_volume = new_volume;
            None
        }
        AudioCommand::ChangeTrackVolume(track_id, new_volume) => {
            if let Some(track) = engine_state
                .tracks
                .iter_mut()
                .find(|track| track.id == track_id)
            {
                track.track_volume = new_volume;
            }
            None
        }
        AudioCommand::ChangeBpm(new_bpm) => {
            engine_state.bpm = new_bpm;
            None
        }
        AudioCommand::SetTrackMute(track_id, is_muted) => {
            if let Some(track) = engine_state
                .tracks
                .iter_mut()
                .find(|track| track.id == track_id)
            {
                track.is_muted = is_muted;
            }
            None
        }
        AudioCommand::SetPlaying {
            is_playing,
            reset_position,
        } => {
            engine_state.is_playing = is_playing;
            if reset_position {
                engine_state.current_step = 0;
                engine_state
                    .audio_producer
                    .try_push(UiCommand::PlayheadPosition(0.0))
                    .ok();
            }
            engine_state
                .audio_producer
                .try_push(if reset_position && !is_playing {
                    UiCommand::PlaybackStopped
                } else {
                    UiCommand::PlayToggled(is_playing)
                })
                .ok();
            None
        }
    }
}

impl AudioCommand {
    /// Commands that transfer heap ownership need one free return-ring slot.
    pub fn needs_retirement_slot(&self) -> bool {
        matches!(
            self,
            Self::ReplaceProjectParts(_) | Self::PreviewSample(_, _)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        audio::SpectrumFrame,
        project::{AudioBlock, AudioBlockType, Project},
    };
    use ringbuf::{traits::Split, HeapRb};

    fn block(length: u32) -> AudioBlock {
        AudioBlock {
            id: AudioBlockID(1),
            track: 1,
            start_step: 0,
            length,
            block_type: AudioBlockType::Mixing,
            is_muted: false,
        }
    }

    #[test]
    fn project_replacement_returns_old_collection_for_app_thread_drop() {
        let (ui_producer, _) = HeapRb::<UiCommand>::new(2).split();
        let (spectrum_producer, _) = HeapRb::<SpectrumFrame>::new(2).split();
        let (retired_audio_producer, _) = HeapRb::<RetiredAudioData>::new(2).split();
        let project = Project {
            name: "test".to_owned(),
            bpm: 120.0,
            master_volume: 1.0,
            audio_blocks: vec![block(2)],
            tracks: Vec::new(),
            patterns: Vec::new(),
        };
        let mut engine = EngineState::from_project(
            project,
            Vec::new(),
            ui_producer,
            spectrum_producer,
            retired_audio_producer,
            44_100.0,
        );

        let retired = handle_command(
            AudioCommand::ReplaceProjectParts(Box::new(AudioProjectUpdate {
                patterns: None,
                tracks: None,
                audio_blocks: Some(vec![block(9)]),
            })),
            &mut engine,
        );

        assert_eq!(engine.audio_blocks[0].length, 9);
        let Some(RetiredAudioData::ProjectUpdate(retired)) = retired else {
            panic!("project replacement must return retired project data")
        };
        assert_eq!(retired.audio_blocks.as_ref().unwrap()[0].length, 2);
    }
}
