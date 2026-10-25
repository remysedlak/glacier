//! An audio engine for sequencing compositions and applying DSP methods

use crate::audio::{
    audio_command::{handle_command, AudioCommand},
    device::find_working_config,
    metering::{push_meter_updates, push_spectrum_frame},
    mixing::mix_voice,
};
use crate::project::*;
use crate::UiCommand;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat,
};

use glacier_dsp::{
    envelope::{flush_denormal, smooth_toward},
    resample::{align_to_frame, frame_advance, frame_lookahead, interpolate},
    spectral::{hann_window, SpectrumAnalyzer},
    utils::{samples_per_step, semitones_to_rate, soft_clip},
    window::window_compensation,
};
use ringbuf::{
    traits::{Consumer, Observer, Producer},
    HeapCons, HeapProd,
};

const SPECTRUM_WINDOW: usize = 2048;
pub const COMMAND_RING_CAPACITY: usize = 64;
pub const SPECTRUM_BINS: usize = SPECTRUM_WINDOW / 2 + 1;
pub type SpectrumFrame = [f32; SPECTRUM_BINS];

/// Playback-only projection of a pattern; names stay exclusively in the app's
/// persistent project model.
#[derive(Clone)]
pub struct AudioPattern {
    pub id: PatternID,
    pub sequences: Vec<Sequence>,
}

/// Playback-only projection of a track. It deliberately omits project name and
/// path strings and shares immutable sample storage with the app.
#[derive(Clone)]
pub struct AudioTrack {
    pub id: TrackID,
    pub channels: u16,
    pub is_muted: bool,
    pub track_volume: f32,
    pub root_note: u8,
    pub samples: std::sync::Arc<[f32]>,
    pub voice_pool: VoicePool,
    pub rms_l: f32,
    pub rms_r: f32,
    pub peak_hold: f32,
}

impl From<&Track> for AudioTrack {
    fn from(track: &Track) -> Self {
        Self {
            id: track.data.id,
            channels: track.data.channels,
            is_muted: track.data.is_muted,
            track_volume: track.data.track_volume,
            root_note: track.data.root_note,
            samples: std::sync::Arc::clone(&track.samples),
            voice_pool: VoicePool::default(),
            rms_l: 0.0,
            rms_r: 0.0,
            peak_hold: 0.0,
        }
    }
}

/// App-prepared changed project collections. The app clones only the changed
/// collection; audio swaps ownership at a buffer boundary and returns the old
/// collection for destruction on the app thread.
pub struct AudioProjectUpdate {
    pub patterns: Option<Vec<AudioPattern>>,
    pub tracks: Option<Vec<AudioTrack>>,
    pub audio_blocks: Option<Vec<AudioBlock>>,
}

impl AudioProjectUpdate {
    pub fn from_parts(
        tracks: Option<&[Track]>,
        patterns: Option<&[PatternData]>,
        audio_blocks: Option<&[AudioBlock]>,
    ) -> Box<Self> {
        Box::new(Self {
            patterns: patterns.map(|items| {
                items
                    .iter()
                    .map(|pattern| AudioPattern {
                        id: pattern.id,
                        sequences: pattern.sequences.clone(),
                    })
                    .collect()
            }),
            tracks: tracks.map(|items| {
                let mut tracks: Vec<_> = items.iter().map(AudioTrack::from).collect();
                tracks.sort_unstable_by_key(|track| track.id);
                tracks
            }),
            audio_blocks: audio_blocks.map(|items| items.to_vec()),
        })
    }
}

/// Heap-owning audio data retired by the callback and destroyed by the app.
pub enum RetiredAudioData {
    ProjectUpdate(Box<AudioProjectUpdate>),
    PreviewSamples(std::sync::Arc<[f32]>),
}

pub mod audio_command;
pub mod device;
pub mod metering;
pub mod mixing;

/// audio logic shared between audio engine and audio commands
pub struct EngineState {
    patterns: Vec<AudioPattern>,
    tracks: Vec<AudioTrack>,
    audio_blocks: Vec<AudioBlock>,
    audio_producer: HeapProd<UiCommand>,
    spectrum_producer: HeapProd<SpectrumFrame>,
    retired_audio_producer: HeapProd<RetiredAudioData>,
    bpm: f32,
    master_volume: f32,
    is_playing: bool,
    preview_samples: std::sync::Arc<[f32]>,
    preview_channels: u16,
    preview_position: f32,
    sample_rate: f32,
    current_step: usize,
}

impl EngineState {
    /// Build playback state from a project loaded by the app.
    fn from_project(
        project: Project,
        tracks: Vec<Track>,
        producer: HeapProd<UiCommand>,
        spectrum_producer: HeapProd<SpectrumFrame>,
        retired_audio_producer: HeapProd<RetiredAudioData>,
        sample_rate: f32,
    ) -> Self {
        Self {
            patterns: project
                .patterns
                .into_iter()
                .map(|pattern| AudioPattern {
                    id: pattern.id,
                    sequences: pattern.sequences,
                })
                .collect(),
            tracks: {
                let mut tracks: Vec<_> = tracks.iter().map(AudioTrack::from).collect();
                tracks.sort_unstable_by_key(|track| track.id);
                tracks
            },
            audio_blocks: project.audio_blocks.clone(),
            audio_producer: producer,
            spectrum_producer,
            retired_audio_producer,
            master_volume: project.master_volume,
            bpm: project.bpm,
            is_playing: false,
            preview_samples: std::sync::Arc::from([]),
            preview_channels: 2,
            sample_rate,
            current_step: project
                .audio_blocks
                .iter()
                .map(|e| e.start_step + e.length)
                .max()
                .unwrap_or(16)
                .max(1) as usize
                - 1,
            preview_position: 0.0,
        }
    }
}

/// State that persists between invocations of the CPAL output callback.
struct AudioCallbackState {
    sample_counter: f32,
    meter_counter: usize,
    master_rms_l: f32,
    master_rms_r: f32,
    master_peak: f32,
    spectrum_buffer: [f32; SPECTRUM_WINDOW],
    spectrum_len: usize,
    spectrum_analyzer: SpectrumAnalyzer,
    hann_window: Vec<f32>,
    window_compensation: f32,
    spectrum_db: [f32; SPECTRUM_BINS],
    pending_command: Option<AudioCommand>,
    pending_retired: Option<RetiredAudioData>,
}

impl AudioCallbackState {
    fn new() -> Self {
        let hann_window = hann_window(SPECTRUM_WINDOW);
        let window_compensation = window_compensation(&hann_window);
        Self {
            sample_counter: 0.0,
            meter_counter: 0,
            master_rms_l: 0.0,
            master_rms_r: 0.0,
            master_peak: 0.0,
            spectrum_buffer: [0.0; SPECTRUM_WINDOW],
            spectrum_len: 0,
            spectrum_analyzer: SpectrumAnalyzer::new(SPECTRUM_WINDOW),
            hann_window,
            window_compensation,
            spectrum_db: [0.0; SPECTRUM_BINS],
            pending_command: None,
            pending_retired: None,
        }
    }
}

/// Initializes the CPAL engine with a project already loaded by the app.
pub fn init(
    mut consumer: HeapCons<AudioCommand>,
    producer: HeapProd<UiCommand>,
    spectrum_producer: HeapProd<SpectrumFrame>,
    retired_audio_producer: HeapProd<RetiredAudioData>,
    project: Project,
    tracks: Vec<Track>,
) -> (cpal::Stream, f32) {
    // error callback
    let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);

    // cpal setup
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("no output device available");
    let default_config = device
        .default_output_config()
        .expect("error getting default config");

    // Probe for a config this device actually accepts — never trust the
    // reported default blindly, since drivers can report configs that fail
    // hw_params validation (seen on SOF/HDA laptops, among others).
    let (stream_config, sample_format) = find_working_config(&device, &default_config);

    let output_channels = stream_config.channels as usize;
    assert!(
        output_channels >= 2,
        "Glacier requires a stereo-or-better output device; got {} channel(s)",
        output_channels
    );

    let mut callback_state = AudioCallbackState::new();

    let mut engine_state = EngineState::from_project(
        project,
        tracks,
        producer,
        spectrum_producer,
        retired_audio_producer,
        stream_config.sample_rate as f32,
    );

    // Build the real stream using whichever format the probe found working.
    let stream = match sample_format {
        SampleFormat::F32 => {
            let cb = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                fill_audio_output(
                    data,
                    output_channels,
                    &mut consumer,
                    &mut engine_state,
                    &mut callback_state,
                    f32::from,
                );
            };
            device.build_output_stream(stream_config, cb, err_fn, None)
        }
        SampleFormat::I16 => {
            let cb = move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                fill_audio_output(
                    data,
                    output_channels,
                    &mut consumer,
                    &mut engine_state,
                    &mut callback_state,
                    f32_to_i16,
                );
            };
            device.build_output_stream(stream_config, cb, err_fn, None)
        }
        SampleFormat::U16 => {
            let cb = move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                fill_audio_output(
                    data,
                    output_channels,
                    &mut consumer,
                    &mut engine_state,
                    &mut callback_state,
                    f32_to_u16,
                );
            };
            device.build_output_stream(stream_config, cb, err_fn, None)
        }
        other => panic!("Unsupported sample format '{other}'"),
    }
    .expect("Failed to build the output stream, even after probing for a working config.");

    // start the output stream and return it
    stream.play().expect("Failed to play the output stream.");
    (stream, stream_config.sample_rate as f32)
}

fn f32_to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16
}

fn f32_to_u16(sample: f32) -> u16 {
    let clamped = sample.clamp(-1.0, 1.0);
    ((clamped * 0.5 + 0.5) * u16::MAX as f32).round() as u16
}

/// Render directly into CPAL's output format. Keeping the mix frame on the
/// stack avoids an intermediate `Vec<f32>` for integer output devices.
fn fill_audio_output<T, Convert>(
    data: &mut [T],
    output_channels: usize,
    consumer: &mut HeapCons<AudioCommand>,
    engine_state: &mut EngineState,
    callback_state: &mut AudioCallbackState,
    convert: Convert,
) where
    Convert: Fn(f32) -> T,
{
    const MAX_COMMANDS_PER_CALLBACK: usize = 64;

    let mut retirement_queue_blocked = false;
    if let Some(retired) = callback_state.pending_retired.take() {
        if let Err(retired) = engine_state.retired_audio_producer.try_push(retired) {
            callback_state.pending_retired = Some(retired);
            retirement_queue_blocked = true;
        }
    }

    if !retirement_queue_blocked {
        for _ in 0..MAX_COMMANDS_PER_CALLBACK {
            let Some(command) = callback_state
                .pending_command
                .take()
                .or_else(|| consumer.try_pop())
            else {
                break;
            };

            if command.needs_retirement_slot()
                && engine_state.retired_audio_producer.vacant_len() == 0
            {
                callback_state.pending_command = Some(command);
                break;
            }

            if let Some(retired) = handle_command(command, engine_state) {
                if let Err(retired) = engine_state.retired_audio_producer.try_push(retired) {
                    // This can only happen if the ring's capacity observation
                    // raced with its consumer. Keep ownership in callback state
                    // and retry next buffer; never destroy it here.
                    callback_state.pending_retired = Some(retired);
                    break;
                }
            }
        }
    }

    if engine_state.is_playing {
        for track in &mut engine_state.tracks {
            if !track.is_muted {
                track.voice_pool.refresh_active_indices();
            }
        }
    }

    for output_frame in data.chunks_mut(output_channels) {
        let mut sample = [0.0f32; 2];

        if engine_state.is_playing {
            for track in &mut engine_state.tracks {
                if !track.is_muted {
                    for i in 0..track.voice_pool.active_indices().len() {
                        let voice = track.voice_pool.active_indices()[i];
                        mix_voice(track, voice, engine_state.master_volume, &mut sample);
                    }
                }
            }
        }

        if !engine_state.preview_samples.is_empty() {
            process_preview(
                &mut engine_state.preview_position,
                &engine_state.preview_samples,
                &mut sample,
                engine_state.master_volume,
                engine_state.preview_channels,
            );
        }

        sample[0] = soft_clip(sample[0]);
        sample[1] = soft_clip(sample[1]);

        callback_state.spectrum_buffer[callback_state.spectrum_len] = (sample[0] + sample[1]) * 0.5;
        callback_state.spectrum_len += 1;
        if callback_state.spectrum_len == SPECTRUM_WINDOW {
            push_spectrum_frame(
                engine_state,
                &mut callback_state.spectrum_analyzer,
                &callback_state.hann_window,
                callback_state.window_compensation,
                &callback_state.spectrum_buffer,
                &mut callback_state.spectrum_db,
            );
            callback_state.spectrum_len = 0;
        }

        callback_state.master_rms_l = flush_denormal(smooth_toward(
            callback_state.master_rms_l,
            sample[0] * sample[0],
            0.01,
        ));
        callback_state.master_rms_r = flush_denormal(smooth_toward(
            callback_state.master_rms_r,
            sample[1] * sample[1],
            0.01,
        ));
        callback_state.master_peak = callback_state
            .master_peak
            .max(sample[0].abs().max(sample[1].abs()));

        output_frame[0] = convert(sample[0]);
        output_frame[1] = convert(sample[1]);
        for channel in &mut output_frame[2..] {
            *channel = convert(0.0);
        }
    }

    let frames = data.len() / output_channels;
    push_meter_updates(
        engine_state,
        &mut callback_state.meter_counter,
        callback_state.master_rms_l,
        callback_state.master_rms_r,
        &mut callback_state.master_peak,
        frames,
    );

    if engine_state.is_playing {
        callback_state.sample_counter += frames as f32;
        let step_duration = samples_per_step(engine_state.sample_rate, engine_state.bpm);
        let beat = engine_state.current_step as f32 + callback_state.sample_counter / step_duration;
        engine_state
            .audio_producer
            .try_push(UiCommand::PlayheadPosition(beat))
            .ok();

        if callback_state.sample_counter >= step_duration {
            callback_state.sample_counter = 0.0;

            let total_steps = engine_state
                .audio_blocks
                .iter()
                .map(|block| block.start_step + block.length)
                .max()
                .unwrap_or(16)
                .max(1) as usize;
            engine_state.current_step = (engine_state.current_step + 1) % total_steps;
            engine_state
                .audio_producer
                .try_push(UiCommand::StepAdvanced(engine_state.current_step))
                .ok();

            for audio_block in &engine_state.audio_blocks {
                if let AudioBlockType::Sample(track_id) = audio_block.block_type {
                    if engine_state.current_step == audio_block.start_step as usize {
                        if let Some(track) = engine_state
                            .tracks
                            .iter_mut()
                            .find(|track| track.id == track_id)
                        {
                            let step_duration =
                                samples_per_step(engine_state.sample_rate, engine_state.bpm);
                            let stop_at = audio_block.length as f32
                                * frame_advance(track.channels)
                                * step_duration;
                            track.voice_pool.spawn(0.0, 1.0, 0.0, 1.0, Some(stop_at));
                        }
                    }
                }
            }

            for audio_block in &engine_state.audio_blocks {
                let AudioBlockType::Pattern(pattern_id) = audio_block.block_type else {
                    continue;
                };
                if engine_state.current_step < audio_block.start_step as usize
                    || engine_state.current_step
                        >= (audio_block.start_step + audio_block.length) as usize
                {
                    continue;
                }

                let local_step = engine_state.current_step - audio_block.start_step as usize;
                if let Some(pattern) = engine_state
                    .patterns
                    .iter()
                    .find(|pattern| pattern.id == pattern_id)
                {
                    for sequence in &pattern.sequences {
                        let Some(note) = sequence.steps.get(local_step) else {
                            continue;
                        };
                        if note.velocity <= 0.0 {
                            continue;
                        }
                        if let Some(track) = engine_state
                            .tracks
                            .iter_mut()
                            .find(|track| track.id == sequence.track_id)
                        {
                            let playback_rate = semitones_to_rate(note.pitch, track.root_note);
                            track.voice_pool.spawn(
                                0.0,
                                playback_rate,
                                0.0,
                                note.velocity / 127.0,
                                None,
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Process the current audio file being previewed into the audio stream
fn process_preview(
    preview_position: &mut f32,
    preview_samples: &[f32],
    sample: &mut [f32],
    master_volume: f32,
    channels: u16,
) {
    let pos = if channels == 1 {
        *preview_position as usize
    } else {
        align_to_frame(*preview_position)
    };
    let lookahead = frame_lookahead(channels);
    if pos + lookahead < preview_samples.len() {
        let (l, r) = interpolate(preview_samples, *preview_position, channels);
        sample[0] += l * master_volume;
        sample[1] += r * master_volume;
        *preview_position += frame_advance(channels);
    }
}
