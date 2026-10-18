//! An audio engine for sequencing compositions and applying DSP methods
use crate::audio::{
    audio_command::{handle_command, AudioCommand},
    device::find_working_config,
};
use crate::project::*;
use crate::UiCommand;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat,
};
use ringbuf::{
    traits::{Consumer, Producer},
    HeapCons, HeapProd,
};

pub mod audio_command;
pub mod device;

/// audio logic shared between audio engine and audio commands
pub struct EngineState {
    patterns: Vec<PatternData>,
    tracks: Vec<Track>,
    audio_blocks: Vec<AudioBlock>,
    audio_producer: HeapProd<UiCommand>,
    bpm: f32,
    master_volume: f32,
    is_playing: bool,
    is_shutting_down: bool,
    project_name: String,
    project_path: String,
    preview_samples: Vec<f32>,
    preview_position: f32,
    sample_rate: f32,
    current_step: usize,
    triggers: Vec<(TrackID, f32, u8)>,
    db: Vec<f32>,
    windowed: Vec<f32>,
}

/// Initializes the CPAL engine with project file data. Returns the audio stream
pub fn init(
    mut consumer: HeapCons<AudioCommand>,
    producer: HeapProd<UiCommand>,
    project_file: Option<String>,
) -> cpal::Stream {
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

    // load project file to memory
    let project: Project = project_file
        .as_deref()
        .and_then(get_project)
        .unwrap_or_default();

    let tracks: Vec<Track> = get_tracks(&project);

    // setup bpm and volume

    let mut shutdown_volume: f32 = 1.00;
    let mut sample_counter: f32 = 0.0; // tracks how many samples passed, to track when a step passes

    // sample RMS/peak callback state
    let mut meter_counter: usize = 0;
    let mut master_rms_l: f32 = 0.0;
    let mut master_rms_r: f32 = 0.0;
    let mut master_peak: f32 = 0.0;

    // store samples for spectrum math over multiple audio callbacks
    let mut spectrum_buffer: Vec<f32> = Vec::with_capacity(2048);
    const SPECTRUM_WINDOW: usize = 2048;

    let spectrum_analyzer = glacier_dsp::SpectrumAnalyzer::new(SPECTRUM_WINDOW);

    let mut engine_state = EngineState {
        patterns: project.patterns,
        tracks,
        audio_blocks: project.audio_blocks.clone(),
        audio_producer: producer,
        master_volume: project.master_volume,
        bpm: project.bpm,
        is_playing: false,
        is_shutting_down: false,
        project_name: project.name.clone(),
        project_path: project_file.unwrap_or_else(Project::default_project_file),
        preview_samples: Vec::new(),
        sample_rate: stream_config.sample_rate as f32,
        current_step: project
            .audio_blocks
            .iter()
            .map(|e| e.start_step + e.length)
            .max()
            .unwrap_or(16) as usize
            - 1,
        preview_position: 0.0,

        // information to not be reallocated during callback
        triggers: Vec::new(),
        db: Vec::new(),
        windowed: Vec::new(),
    };

    // load sample rate (the ACTUAL validated rate, not the reported default)
    engine_state
        .audio_producer
        .try_push(UiCommand::SampleRateLoaded(engine_state.sample_rate))
        .ok();

    engine_state
        .audio_producer
        .try_push(UiCommand::LoadProject {
            tracks: engine_state.tracks.clone(),
            patterns: engine_state.patterns.clone(),
            audio_blocks: engine_state.audio_blocks.clone(),
            bpm: engine_state.bpm,
            master_volume: engine_state.master_volume,
            project_path: engine_state.project_path.clone(),
        })
        .ok();

    // audio callback
    // fills samples requested from CPAL audio driver
    let mut fill_f32 = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        // parse incoming UI commands before fulfilling data callback
        while let Some(cmd) = consumer.try_pop() {
            handle_command(cmd, &mut engine_state);
        }

        // for each sample requested, mix in the appropriate track samples
        for sample in data.chunks_mut(2) {
            // fade audio off during app shutdown
            if engine_state.is_shutting_down {
                shutdown_volume -= 0.0001;
                if shutdown_volume <= 0.0 {
                    engine_state
                        .audio_producer
                        .try_push(UiCommand::ShutdownComplete)
                        .ok();
                }
            }

            // Zero out the sample. Fill it if the song currently is_playing.
            sample[0] = 0.0; // left channel
            sample[1] = 0.0; // right channel

            if engine_state.is_playing {
                for track in &mut engine_state.tracks {
                    if !track.data.is_muted {
                        for voice in &mut track.voices {
                            if voice.is_playing {
                                let pos = align_to_frame(voice.position);
                                let frac = voice.position - voice.position.floor();

                                let stop =
                                    voice.stop_at_frame.unwrap_or(track.samples.len() as f32);
                                if pos as f32 + 3.0 >= stop.min(track.samples.len() as f32) {
                                    voice.is_playing = false;
                                } else {
                                    voice.current_volume = glacier_dsp::smooth_toward(
                                        voice.current_volume,
                                        voice.target_volume,
                                        0.01,
                                    );

                                    let l = track.samples[pos]
                                        + frac * (track.samples[pos + 2] - track.samples[pos]);
                                    let r = track.samples[pos + 1]
                                        + frac * (track.samples[pos + 3] - track.samples[pos + 1]);

                                    let gain = voice.current_volume
                                        * track.data.track_volume
                                        * shutdown_volume
                                        * engine_state.master_volume;
                                    sample[0] += l * gain;
                                    sample[1] += r * gain;

                                    voice.position += 2.0 * voice.playback_rate;

                                    track.rms_l =
                                        glacier_dsp::smooth_toward(track.rms_l, l * l, 0.01);
                                    track.rms_r =
                                        glacier_dsp::smooth_toward(track.rms_r, r * r, 0.01);
                                    track.peak_hold = track.peak_hold.max(l.abs().max(r.abs()));
                                }
                            }
                        }
                        track.voices.retain(|v| v.is_playing);
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
                );
            }

            // feed the spectrum analyzer with the final mixed mono signal
            spectrum_buffer.push((sample[0] + sample[1]) * 0.5);

            // update master meter info
            master_rms_l = glacier_dsp::smooth_toward(master_rms_l, sample[0] * sample[0], 0.01);
            master_rms_r = glacier_dsp::smooth_toward(master_rms_r, sample[1] * sample[1], 0.01);
            master_peak = master_peak.max(sample[0].abs().max(sample[1].abs()));
        }

        // update the meter data
        meter_counter += data.len() / 2;
        if meter_counter >= 1024 {
            meter_counter = 0;
            engine_state
                .audio_producer
                .try_push(UiCommand::MasterLevel(
                    master_rms_l.sqrt(),
                    master_rms_r.sqrt(),
                    master_peak,
                ))
                .ok();
            master_peak = 0.0;
            for track in &mut engine_state.tracks {
                engine_state
                    .audio_producer
                    .try_push(UiCommand::TrackLevel(
                        track.data.id,
                        track.rms_l.sqrt(),
                        track.rms_r.sqrt(),
                        track.peak_hold,
                    ))
                    .ok();
                track.peak_hold = 0.0;
            }
        } // exit sample mixing

        // when the spectrum buffer is full, push fourier compute to ui
        if spectrum_buffer.len() >= SPECTRUM_WINDOW {
            let window = glacier_dsp::hann_window(SPECTRUM_WINDOW);
            let compensation = glacier_dsp::window_compensation(&window);

            engine_state.windowed.clear();
            engine_state.windowed.extend(
                spectrum_buffer
                    .iter()
                    .zip(window.iter())
                    .map(|(x, w)| x * w),
            );

            let magnitudes = spectrum_analyzer.process(&engine_state.windowed);
            engine_state.db.clear();
            engine_state.db.extend(
                magnitudes
                    .iter()
                    .map(|m| glacier_dsp::magnitude_to_db(*m, SPECTRUM_WINDOW, compensation)),
            );

            engine_state
                .audio_producer
                .try_push(UiCommand::SpectrumFrame(std::mem::take(
                    &mut engine_state.db,
                )))
                .ok();

            spectrum_buffer.clear();
        }

        if engine_state.is_playing {
            sample_counter += data.len() as f32 / 2.0; // increment sample counter by number of samples requested : keep track of sample position

            // get amount of samples per step
            let samples_per_step =
                glacier_dsp::samples_per_step(engine_state.sample_rate, engine_state.bpm);

            // update UI time
            let beat = engine_state.current_step as f32 + (sample_counter / samples_per_step);
            engine_state
                .audio_producer
                .try_push(UiCommand::PlayheadPosition(beat))
                .ok();

            // increment the step if enough samples have passed
            if sample_counter >= samples_per_step {
                sample_counter = 0.0;

                let total_steps = engine_state
                    .audio_blocks
                    .iter()
                    .map(|e| e.start_step + e.length)
                    .max()
                    .unwrap_or(16) as usize;
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
                                let samples_per_step = glacier_dsp::samples_per_step(
                                    engine_state.sample_rate,
                                    engine_state.bpm,
                                );
                                let stop_at = audio_block.length as f32 * samples_per_step * 2.0;

                                track.voices.push(Voice {
                                    position: 0.0,
                                    is_playing: true,
                                    playback_rate: 1.0,
                                    current_volume: 0.0,
                                    target_volume: 1.0,
                                    stop_at_frame: Some(stop_at),
                                });
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
                        let playback_rate =
                            glacier_dsp::semitones_to_rate(pitch, track.data.root_note);
                        track.voices.push(Voice {
                            position: 0.0,
                            is_playing: true,
                            playback_rate,
                            current_volume: 0.0,
                            target_volume: velocity / 127.0,
                            stop_at_frame: None,
                        });
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
            device.build_output_stream(&stream_config, cb, err_fn, None)
        }
        SampleFormat::I16 => {
            let mut scratch: Vec<f32> = Vec::new();
            let cb = move |data: &mut [i16], info: &cpal::OutputCallbackInfo| {
                scratch.resize(data.len(), 0.0);
                fill_f32(&mut scratch, info);
                for (o, s) in data.iter_mut().zip(scratch.iter()) {
                    *o = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                }
            };
            device.build_output_stream(&stream_config, cb, err_fn, None)
        }
        SampleFormat::U16 => {
            let mut scratch: Vec<f32> = Vec::new();
            let cb = move |data: &mut [u16], info: &cpal::OutputCallbackInfo| {
                scratch.resize(data.len(), 0.0);
                fill_f32(&mut scratch, info);
                for (o, s) in data.iter_mut().zip(scratch.iter()) {
                    let clamped = s.clamp(-1.0, 1.0);
                    *o = ((clamped * 0.5 + 0.5) * u16::MAX as f32) as u16;
                }
            };
            device.build_output_stream(&stream_config, cb, err_fn, None)
        }
        other => panic!("Unsupported sample format '{other}'"),
    }
    .expect("Failed to build the output stream, even after probing for a working config.");

    // start the output stream and return it
    stream.play().expect("Failed to play the output stream.");
    stream
}

/// Process the current audio file being previewed into the audio stream
fn process_preview(
    preview_position: &mut f32,
    preview_samples: &[f32],
    sample: &mut [f32],
    master_volume: f32,
) {
    let pos = align_to_frame(*preview_position);
    if pos + 3 < preview_samples.len() {
        let frac = *preview_position - preview_position.floor();
        let l = preview_samples[pos] + frac * (preview_samples[pos + 2] - preview_samples[pos]);
        let r =
            preview_samples[pos + 1] + frac * (preview_samples[pos + 3] - preview_samples[pos + 1]);
        sample[0] += l * master_volume;
        sample[1] += r * master_volume;
        *preview_position += 2.0;
    }
}

/// even out a sample position to force stereo operations on even array
fn align_to_frame(pos: f32) -> usize {
    (pos as usize) & !1
}
