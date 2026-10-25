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
    traits::{Consumer, Producer},
    HeapCons, HeapProd,
};

const SPECTRUM_WINDOW: usize = 2048;

pub mod audio_command;
pub mod device;
pub mod metering;
pub mod mixing;

/// audio logic shared between audio engine and audio commands
pub struct EngineState {
    patterns: Vec<PatternData>,
    tracks: Vec<Track>,
    audio_blocks: Vec<AudioBlock>,
    audio_producer: HeapProd<UiCommand>,
    bpm: f32,
    master_volume: f32,
    is_playing: bool,
    preview_samples: Vec<f32>,
    preview_channels: u16,
    preview_position: f32,
    sample_rate: f32,
    current_step: usize,
    triggers: Vec<(TrackID, f32, u8)>,
    db: Vec<f32>,
    windowed: Vec<f32>,
}

impl EngineState {
    /// Build playback state from a project loaded by the app.
    fn from_project(
        project: Project,
        tracks: Vec<Track>,
        producer: HeapProd<UiCommand>,
        sample_rate: f32,
    ) -> Self {
        let max_triggers = project.patterns.iter().map(|p| p.sequences.len()).sum();

        Self {
            patterns: project.patterns,
            tracks,
            audio_blocks: project.audio_blocks.clone(),
            audio_producer: producer,
            master_volume: project.master_volume,
            bpm: project.bpm,
            is_playing: false,
            preview_samples: Vec::new(),
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
            // Keep these buffers ready for callback work.
            triggers: Vec::with_capacity(max_triggers),
            db: Vec::new(),
            windowed: Vec::new(),
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
    spectrum_buffer: Vec<f32>,
    spectrum_analyzer: SpectrumAnalyzer,
    hann_window: Vec<f32>,
    window_compensation: f32,
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
            spectrum_buffer: Vec::with_capacity(SPECTRUM_WINDOW),
            spectrum_analyzer: SpectrumAnalyzer::new(SPECTRUM_WINDOW),
            hann_window,
            window_compensation,
        }
    }
}

/// Initializes the CPAL engine with a project already loaded by the app.
pub fn init(
    mut consumer: HeapCons<AudioCommand>,
    producer: HeapProd<UiCommand>,
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

    let mut engine_state =
        EngineState::from_project(project, tracks, producer, stream_config.sample_rate as f32);

    // audio callback
    // fills samples requested from CPAL audio driver
    let mut fill_f32 = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        // Apply queued commands before rendering the next audio block.
        while let Some(cmd) = consumer.try_pop() {
            handle_command(cmd, &mut engine_state);
        }

        // refresh each track's active-voice index cache once per callback —
        // NOT once per sample (see VoicePool::refresh_active_indices docs)
        if engine_state.is_playing {
            for track in &mut engine_state.tracks {
                if !track.data.is_muted {
                    track.voice_pool.refresh_active_indices();
                }
            }
        }

        // for each sample requested, mix in the appropriate track samples
        for sample in data.chunks_mut(output_channels) {
            // Zero out the sample. Fill it if the song currently is_playing.
            sample[0] = 0.0; // left channel
            sample[1] = 0.0; // right channel

            // Output beyond stereo (channels 2+) is intentionally silent.
            // Glacier mixes internally in stereo only.
            // Real multichannel routing is a future feature, not implemented here.
            for ch in 2..output_channels {
                sample[ch] = 0.0;
            }

            if engine_state.is_playing {
                for track in &mut engine_state.tracks {
                    if !track.data.is_muted {
                        for i in 0..track.voice_pool.active_indices().len() {
                            let voice = track.voice_pool.active_indices()[i]; // short-lived borrow, ends immediately
                            mix_voice(track, voice, engine_state.master_volume, sample);
                        }
                    }
                }
            }

            // preview playback at master volume (user clicks a file on file tree or track window)
            if !engine_state.preview_samples.is_empty() {
                process_preview(
                    &mut engine_state.preview_position,
                    &engine_state.preview_samples,
                    sample,
                    engine_state.master_volume,
                    engine_state.preview_channels,
                );
            }

            // limit the final mixed signal before anything reads it
            sample[0] = soft_clip(sample[0]);
            sample[1] = soft_clip(sample[1]);

            // feed the spectrum analyzer with the final mixed mono signal
            callback_state
                .spectrum_buffer
                .push((sample[0] + sample[1]) * 0.5);

            // update master meter info
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
        }

        // update the meter data
        push_meter_updates(
            &mut engine_state,
            &mut callback_state.meter_counter,
            callback_state.master_rms_l,
            callback_state.master_rms_r,
            &mut callback_state.master_peak,
            data.len() / output_channels,
        );

        // when the spectrum buffer is full, push fourier compute to ui
        push_spectrum_frame(
            &mut engine_state,
            &callback_state.spectrum_analyzer,
            &callback_state.hann_window,
            callback_state.window_compensation,
            &mut callback_state.spectrum_buffer,
        );

        if engine_state.is_playing {
            callback_state.sample_counter += data.len() as f32 / output_channels as f32;

            // get amount of samples per step
            let step_duration: f32 = samples_per_step(engine_state.sample_rate, engine_state.bpm);

            // update UI time
            let beat =
                engine_state.current_step as f32 + (callback_state.sample_counter / step_duration);
            engine_state
                .audio_producer
                .try_push(UiCommand::PlayheadPosition(beat))
                .ok();

            // increment the step if enough samples have passed
            if callback_state.sample_counter >= step_duration {
                callback_state.sample_counter = 0.0;

                let total_steps = engine_state
                    .audio_blocks
                    .iter()
                    .map(|e| e.start_step + e.length)
                    .max()
                    .unwrap_or(16)
                    .max(1) as usize;
                engine_state.current_step = (engine_state.current_step + 1) % total_steps;

                engine_state
                    .audio_producer
                    .try_push(UiCommand::StepAdvanced(engine_state.current_step))
                    .ok();

                // build out each note
                engine_state.triggers.clear();
                engine_state.triggers.extend(
                    engine_state
                        .audio_blocks
                        .iter()
                        .filter_map(|e| {
                            if let AudioBlockType::Pattern(pattern_id) = e.block_type {
                                if engine_state.current_step >= e.start_step as usize
                                    && engine_state.current_step
                                        < (e.start_step + e.length) as usize
                                {
                                    let local_step =
                                        engine_state.current_step - e.start_step as usize;
                                    return engine_state
                                        .patterns
                                        .iter()
                                        .find(|p| p.id == pattern_id)
                                        .map(|p| (p, local_step));
                                }
                            }

                            None
                        })
                        .flat_map(|(p, local_step)| {
                            p.sequences
                                .iter()
                                .filter(move |s| {
                                    local_step < s.steps.len() && s.steps[local_step].velocity > 0.0
                                })
                                .map(move |s| {
                                    let note = &s.steps[local_step];
                                    (s.track_id, note.velocity, note.pitch)
                                })
                        }),
                );

                for audio_block in &engine_state.audio_blocks {
                    if let AudioBlockType::Sample(track_id) = audio_block.block_type {
                        if engine_state.current_step == audio_block.start_step as usize {
                            if let Some(track) = engine_state
                                .tracks
                                .iter_mut()
                                .find(|t| t.data.id == track_id)
                            {
                                let step_duration: f32 =
                                    samples_per_step(engine_state.sample_rate, engine_state.bpm);
                                let stop_at = audio_block.length as f32
                                    * frame_advance(track.data.channels)
                                    * step_duration;

                                track.voice_pool.spawn(0.0, 1.0, 0.0, 1.0, Some(stop_at));
                            }
                        }
                    }
                }

                for &(track_id, velocity, pitch) in &engine_state.triggers {
                    if let Some(track) = engine_state
                        .tracks
                        .iter_mut()
                        .find(|track| track.data.id == track_id)
                    {
                        let playback_rate = semitones_to_rate(pitch, track.data.root_note);
                        track
                            .voice_pool
                            .spawn(0.0, playback_rate, 0.0, velocity / 127.0, None);
                    }
                }
            }
        }
    };

    // Build the real stream using whichever format the probe found working.
    let stream = match sample_format {
        SampleFormat::F32 => {
            let cb = move |data: &mut [f32], info: &cpal::OutputCallbackInfo| {
                fill_f32(data, info);
            };
            device.build_output_stream(stream_config, cb, err_fn, None)
        }
        SampleFormat::I16 => {
            let mut scratch: Vec<f32> = Vec::new();
            let cb = move |data: &mut [i16], info: &cpal::OutputCallbackInfo| {
                scratch.resize(data.len(), 0.0);
                fill_f32(&mut scratch, info);
                for (o, s) in data.iter_mut().zip(scratch.iter()) {
                    *o = (s.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16;
                }
            };
            device.build_output_stream(stream_config, cb, err_fn, None)
        }
        SampleFormat::U16 => {
            let mut scratch: Vec<f32> = Vec::new();
            let cb = move |data: &mut [u16], info: &cpal::OutputCallbackInfo| {
                scratch.resize(data.len(), 0.0);
                fill_f32(&mut scratch, info);
                for (o, s) in data.iter_mut().zip(scratch.iter()) {
                    let clamped = s.clamp(-1.0, 1.0);
                    *o = ((clamped * 0.5 + 0.5) * u16::MAX as f32).round() as u16;
                }
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
