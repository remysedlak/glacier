use crate::app::UiCommand;
use crate::audio::{audio_command::AudioCommand, RetiredAudioData, SpectrumFrame};
use cpal::Stream;
use ringbuf::{
    traits::Producer,
    {HeapCons, HeapProd},
};
use std::collections::VecDeque;

/// UI-side sender that retains commands when the audio ring buffer is full.
/// Commands stay ordered and are retried on subsequent app events/draws.
pub(crate) struct AudioCommandSender {
    ring: HeapProd<AudioCommand>,
    pending: VecDeque<AudioCommand>,
}

const MAX_PENDING_AUDIO_COMMANDS: usize = 256;

impl AudioCommandSender {
    pub(super) fn new(ring: HeapProd<AudioCommand>) -> Self {
        Self {
            ring,
            pending: VecDeque::with_capacity(MAX_PENDING_AUDIO_COMMANDS),
        }
    }

    pub(crate) fn try_push(&mut self, mut command: AudioCommand) -> Result<(), AudioCommand> {
        if !self.pending.is_empty() {
            self.coalesce_pending(&mut command);
            if self.pending.len() >= MAX_PENDING_AUDIO_COMMANDS {
                log::error!("audio command backlog reached its bounded capacity; command rejected");
                return Err(command);
            }
            self.pending.push_back(command);
            return Ok(());
        }

        match self.ring.try_push(command) {
            Ok(()) => Ok(()),
            Err(command) => {
                if self.pending.len() >= MAX_PENDING_AUDIO_COMMANDS {
                    log::error!(
                        "audio command backlog reached its bounded capacity; command rejected"
                    );
                    return Err(command);
                }
                self.pending.push_back(command);
                Ok(())
            }
        }
    }

    fn coalesce_pending(&mut self, incoming: &mut AudioCommand) {
        if let AudioCommand::ReplaceProjectParts(new_update) = incoming {
            let mut tracks_base = None;
            let mut blocks_base = None;
            for (index, queued) in self.pending.iter_mut().enumerate().rev() {
                if let AudioCommand::ReplaceProjectParts(old_update) = queued {
                    if new_update.patterns.is_none() {
                        new_update.patterns = old_update.patterns.take();
                    }
                    if new_update.tracks.is_none() {
                        if old_update.tracks.is_some() {
                            new_update.tracks = old_update.tracks.take();
                            tracks_base = Some(index);
                        }
                    }
                    if new_update.audio_blocks.is_none() {
                        if old_update.audio_blocks.is_some() {
                            new_update.audio_blocks = old_update.audio_blocks.take();
                            blocks_base = Some(index);
                        }
                    }
                }
            }

            // Moving a coalesced collection update after a scalar command must
            // retain that command's newest value. Replay only commands that
            // were queued after the update supplying each collection.
            if let (Some(base), Some(tracks)) = (tracks_base, new_update.tracks.as_mut()) {
                for queued in self.pending.iter().skip(base + 1) {
                    match queued {
                        AudioCommand::ChangeTrackVolume(id, volume) => {
                            if let Some(track) = tracks.iter_mut().find(|track| track.id == *id) {
                                track.track_volume = *volume;
                            }
                        }
                        AudioCommand::SetTrackMute(id, is_muted) => {
                            if let Some(track) = tracks.iter_mut().find(|track| track.id == *id) {
                                track.is_muted = *is_muted;
                            }
                        }
                        _ => {}
                    }
                }
            }
            if let (Some(base), Some(blocks)) = (blocks_base, new_update.audio_blocks.as_mut()) {
                for queued in self.pending.iter().skip(base + 1) {
                    if let AudioCommand::ResizeAudioBlock(id, length) = queued {
                        if let Some(block) = blocks.iter_mut().find(|block| block.id == *id) {
                            block.length = *length;
                        }
                    }
                }
            }
            self.pending
                .retain(|queued| !matches!(queued, AudioCommand::ReplaceProjectParts(_)));
            return;
        }

        if let AudioCommand::SetPlaying { reset_position, .. } = incoming {
            for queued in self.pending.iter_mut().rev() {
                if let AudioCommand::SetPlaying {
                    reset_position: old_reset,
                    ..
                } = queued
                {
                    *reset_position |= *old_reset;
                }
            }
        }

        self.pending.retain(|queued| match (&*queued, &*incoming) {
            (AudioCommand::ChangeBpm(_), AudioCommand::ChangeBpm(_))
            | (AudioCommand::ChangeMasterVolume(_), AudioCommand::ChangeMasterVolume(_))
            | (AudioCommand::PreviewSample(_, _), AudioCommand::PreviewSample(_, _))
            | (AudioCommand::SetPlaying { .. }, AudioCommand::SetPlaying { .. }) => false,
            (
                AudioCommand::ChangeTrackVolume(old_id, _),
                AudioCommand::ChangeTrackVolume(new_id, _),
            )
            | (AudioCommand::SetTrackMute(old_id, _), AudioCommand::SetTrackMute(new_id, _)) => {
                old_id != new_id
            }
            (
                AudioCommand::ResizeAudioBlock(old_id, _),
                AudioCommand::ResizeAudioBlock(new_id, _),
            ) => old_id != new_id,
            _ => true,
        });
    }

    pub(crate) fn flush_pending(&mut self) {
        while let Some(command) = self.pending.pop_front() {
            if let Err(command) = self.ring.try_push(command) {
                self.pending.push_front(command);
                break;
            }
        }
    }

    pub(super) fn replace_ring(&mut self, ring: HeapProd<AudioCommand>) {
        self.pending.clear();
        self.ring = ring;
    }
}
pub(super) struct AudioBridge {
    pub(super) commands: AudioCommandSender,
    pub(super) ui_commands: HeapCons<UiCommand>,
    pub(super) stream: Option<Stream>,
    pub(super) audio_commands: Option<HeapCons<AudioCommand>>,
    pub(super) ui_producer: Option<HeapProd<UiCommand>>,
    pub(super) spectrum_consumer: HeapCons<SpectrumFrame>,
    pub(super) spectrum_producer: Option<HeapProd<SpectrumFrame>>,
    pub(super) retired_audio_consumer: HeapCons<RetiredAudioData>,
    pub(super) retired_audio_producer: Option<HeapProd<RetiredAudioData>>,
}

impl AudioBridge {
    pub(super) fn new(
        commands: HeapProd<AudioCommand>,
        audio_commands: HeapCons<AudioCommand>,
        ui_producer: HeapProd<UiCommand>,
        ui_commands: HeapCons<UiCommand>,
        spectrum_producer: HeapProd<SpectrumFrame>,
        spectrum_consumer: HeapCons<SpectrumFrame>,
        retired_audio_producer: HeapProd<RetiredAudioData>,
        retired_audio_consumer: HeapCons<RetiredAudioData>,
    ) -> Self {
        Self {
            commands: AudioCommandSender::new(commands),
            ui_commands,
            stream: None,
            audio_commands: Some(audio_commands),
            ui_producer: Some(ui_producer),
            spectrum_consumer,
            spectrum_producer: Some(spectrum_producer),
            retired_audio_consumer,
            retired_audio_producer: Some(retired_audio_producer),
        }
    }
}

#[cfg(test)]
mod audio_command_sender_tests {
    use super::*;
    use crate::{
        audio::{AudioPattern, AudioProjectUpdate},
        project::{AudioBlock, AudioBlockID, AudioBlockType, PatternID, TrackID},
    };
    use ringbuf::{
        traits::{Consumer, Producer, Split},
        HeapRb,
    };

    fn block(length: u32) -> AudioBlock {
        AudioBlock {
            id: AudioBlockID(7),
            track: 1,
            start_step: 0,
            length,
            block_type: AudioBlockType::Mixing,
            is_muted: false,
        }
    }

    #[test]
    fn merged_project_update_keeps_resize_queued_after_its_snapshot() {
        let (mut ring_producer, mut ring_consumer) = HeapRb::<AudioCommand>::new(1).split();
        assert!(ring_producer
            .try_push(AudioCommand::ChangeBpm(120.0))
            .is_ok());
        let mut sender = AudioCommandSender::new(ring_producer);

        assert!(sender
            .try_push(AudioCommand::ReplaceProjectParts(Box::new(
                AudioProjectUpdate {
                    patterns: None,
                    tracks: None,
                    audio_blocks: Some(vec![block(2)]),
                },
            )))
            .is_ok());
        assert!(sender
            .try_push(AudioCommand::ResizeAudioBlock(AudioBlockID(7), 9))
            .is_ok());
        assert!(sender
            .try_push(AudioCommand::ReplaceProjectParts(Box::new(
                AudioProjectUpdate {
                    patterns: Some(vec![AudioPattern {
                        id: PatternID(3),
                        sequences: Vec::new(),
                    }]),
                    tracks: None,
                    audio_blocks: None,
                },
            )))
            .is_ok());

        assert!(matches!(
            ring_consumer.try_pop(),
            Some(AudioCommand::ChangeBpm(_))
        ));
        sender.flush_pending();
        assert!(matches!(
            ring_consumer.try_pop(),
            Some(AudioCommand::ResizeAudioBlock(AudioBlockID(7), 9))
        ));
        sender.flush_pending();
        let Some(AudioCommand::ReplaceProjectParts(update)) = ring_consumer.try_pop() else {
            panic!("expected merged project update")
        };
        assert_eq!(update.patterns.as_ref().unwrap()[0].id, PatternID(3));
        assert_eq!(update.audio_blocks.as_ref().unwrap()[0].length, 9);
        assert!(sender.pending.is_empty());
    }

    #[test]
    fn pending_scalar_controls_keep_only_the_latest_value_for_each_target() {
        let (ring_producer, _ring_consumer) = HeapRb::<AudioCommand>::new(1).split();
        let mut sender = AudioCommandSender::new(ring_producer);
        assert!(sender
            .try_push(AudioCommand::ChangeTrackVolume(TrackID(2), 0.4))
            .is_ok());
        assert!(sender
            .try_push(AudioCommand::ChangeTrackVolume(TrackID(2), 0.8))
            .is_ok());
        assert!(sender
            .try_push(AudioCommand::SetTrackMute(TrackID(2), true))
            .is_ok());
        assert!(sender
            .try_push(AudioCommand::SetTrackMute(TrackID(2), false))
            .is_ok());

        assert_eq!(sender.pending.len(), 2);
        assert!(matches!(
            sender.pending[0],
            AudioCommand::ChangeTrackVolume(TrackID(2), 0.8)
        ));
        assert!(matches!(
            sender.pending[1],
            AudioCommand::SetTrackMute(TrackID(2), false)
        ));
    }
}
