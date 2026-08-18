//! audio engine for sequencing compositions and applying DSP
use crate::project::*;
use crate::UiCommand;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    {SampleFormat, Stream},
};
use ringbuf::{
    traits::{Consumer, Producer},
    HeapCons, HeapProd,
};

/// commands retrieved from the user interface to control the audio engine
pub enum AudioCommand {
    // composition details
    ToggleStep(usize, u32, usize),
    ToggleNote(usize, u32, usize, u8), // pattern_id, track_id, step_idx, pitch
    ChangeBpm(f32),
    DeleteAudioBlock(usize),
    CreateAudioBlock(usize, u32, usize, AudioBlockType),
    ResizeAudioBlock(usize, u32),

    // mixing
    ChangeMasterVolume(f32),
    ToggleTrackMute(usize),
    ChangeTrackVolume(usize, f32),

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
    DuplicatePattern(usize),
    AddPattern,
    DeletePattern(usize),
    ClearPattern(usize),

    // renaming state
    RenamePattern(usize, String),
    RenameTrack(usize, String),

    // tracks
    LoadTrack(TrackData, Vec<f32>),
    DeleteTrack(usize),
}

/// initialize the CPAL engine with project file data and return the audio stream
pub fn init(
    mut consumer: HeapCons<AudioCommand>,
    mut producer: HeapProd<UiCommand>,
    project_file: Option<String>,
) -> Stream {
    // error callback
    let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);

    // cpal setup -> host, device, config
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("no output device available");
    let supported_config = device
        .default_output_config()
        .expect("error getting default config");
    let config = supported_config.config();
    let sample_format = supported_config.sample_format();

    // load project file to memory
    let project = project_file
        .as_deref()
        .and_then(get_project)
        .unwrap_or_default();

    let mut project_path = project_file.unwrap_or_else(|| Project::default_project_file());
    let mut tracks: Vec<Track> = get_tracks(&project);
    let mut patterns = project.patterns;
    let mut events = project.events;

    // load sample rate
    producer
        .try_push(UiCommand::SampleRateLoaded(config.sample_rate as f32))
        .ok();

    // setup bpm and volume

    let mut bpm: f32 = project.bpm;
    let mut master_volume = project.master_volume;
    let mut current_step = events
        .iter()
        .map(|e| e.start_step + e.length)
        .max()
        .unwrap_or(16) as usize
        - 1;
    let mut is_playing = false; // step function
    let mut is_shutting_down = false;
    let mut shutdown_volume: f32 = 1.00;
    let mut sample_counter: f32 = 0.0; // tracks how many samples passed, to track when a step passes
    let name = project.name.clone();

    // sample RMS/peak callback state
    let mut meter_counter: usize = 0;
    let mut master_rms_l: f32 = 0.0;
    let mut master_rms_r: f32 = 0.0;
    let mut master_peak: f32 = 0.0;

    // store samples for spectrum math over multiple audio callbacks
    let mut spectrum_buffer: Vec<f32> = Vec::with_capacity(2048);
    const SPECTRUM_WINDOW: usize = 2048;

    let mut preview_samples: Vec<f32> = Vec::new();
    let mut preview_position: f32 = 0.0;

    producer
        .try_push(UiCommand::LoadProject {
            tracks: tracks.clone(),
            patterns: patterns.clone(),
            events: events.clone(),
            bpm,
            master_volume,
            project_path: project_path.clone(),
        })
        .ok();

    // audio callback
    // fills samples requested from CPAL audio driver
    let sequencer_callback = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        // parse incoming UI commands before fulfilling data callback
        while let Some(cmd) = consumer.try_pop() {
            match cmd {
                // clear a patterns step data by looping through each sequencer and emptying the note
                AudioCommand::ClearPattern(pattern_id) => {
                    if let Some(pattern) = patterns.iter_mut().find(|p| p.id == pattern_id) {
                        for seq in pattern.sequences.iter_mut() {
                            for note in seq.steps.iter_mut() {
                                *note = Note::DEFAULT;
                            }
                        }
                        // update the ui
                        producer
                            .try_push(UiCommand::PatternLoaded(pattern.clone()))
                            .ok();
                    }
                }
                AudioCommand::ShutdownWithoutSaving => {
                    if !is_playing {
                        producer.try_push(UiCommand::ShutdownComplete).ok();
                    }
                    is_shutting_down = true;
                }
                AudioCommand::RenamePattern(pattern_id, name) => {
                    if let Some(pattern) = patterns.iter_mut().find(|p| p.id == pattern_id) {
                        pattern.name = name.clone();
                        producer
                            .try_push(UiCommand::PatternRenamed(pattern_id, name))
                            .ok();
                    }
                }
                AudioCommand::RenameTrack(track_id, name) => {
                    if let Some(track) = tracks.iter_mut().find(|t| t.data.id == track_id as u32) {
                        track.data.name = name.clone();
                        producer
                            .try_push(UiCommand::TrackRenamed(track.data.id, name))
                            .ok();
                    }
                }
                AudioCommand::PreviewSample(samples) => {
                    preview_samples = samples;
                    preview_position = 0.0;
                }
                AudioCommand::SetProjectPath(new_path) => project_path = new_path,
                AudioCommand::ResizeAudioBlock(event_id, new_length) => {
                    if let Some(event) = events.iter_mut().find(|event| event.id == event_id) {
                        event.length = new_length;
                    }
                }
                AudioCommand::DuplicatePattern(pattern_id) => {
                    if let Some(pattern) = patterns.iter().find(|p| p.id == pattern_id).cloned() {
                        let new_pattern = pattern.duplicate(
                            patterns
                                .iter()
                                .map(|x| x.id)
                                .max()
                                .map(|m| m + 1)
                                .unwrap_or(0),
                        );
                        patterns.push(new_pattern.clone());
                        // update the ui
                        producer
                            .try_push(UiCommand::PatternLoaded(new_pattern))
                            .ok();
                    }
                }

                // activate a step within a track within a pattern
                AudioCommand::ToggleNote(pattern_id, track_id, step_idx, pitch) => {
                    if let Some(pattern) = patterns.iter_mut().find(|p| p.id == pattern_id) {
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
                AudioCommand::AddPattern => {
                    let new_pattern_id = patterns
                        .iter()
                        .map(|x| x.id)
                        .max()
                        .map(|m| m + 1)
                        .unwrap_or(0);

                    // create name to be Nth pattern available
                    let name = format!("Pattern {}", patterns.len() + 1);

                    let sequences = tracks
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
                    patterns.push(p.clone());
                    producer.try_push(UiCommand::PatternLoaded(p)).ok();
                }
                AudioCommand::DeletePattern(pattern_id) => {
                    // remove the pattern from list of patterns
                    patterns.retain(|p| p.id != pattern_id);
                    // remove the pattern from list of events
                    events.retain(|e| {
                        if let AudioBlockType::Pattern(pid) = e.block_type {
                            pid != pattern_id
                        } else {
                            true
                        }
                    });
                    producer
                        .try_push(UiCommand::PatternDeleted(pattern_id))
                        .ok();
                }
                AudioCommand::DeleteAudioBlock(audio_block_id) => {
                    events.retain(|e| e.id != audio_block_id);
                    producer
                        .try_push(UiCommand::AudioBlockDeleted(audio_block_id))
                        .ok();
                }
                AudioCommand::Stop => {
                    is_playing = false;
                    current_step = 0;
                }
                AudioCommand::CreateAudioBlock(track, start_step, length, block_type) => {
                    // add new event to playlist
                    let audio_block = AudioBlock {
                        id: events
                            .iter()
                            .map(|x| x.id)
                            .max()
                            .map(|m| m + 1)
                            .unwrap_or(0),
                        track,
                        start_step,
                        length: length as u32,
                        block_type,
                    };
                    events.push(audio_block.clone());
                    producer
                        .try_push(UiCommand::AudioBlockLoaded(audio_block))
                        .ok();
                }
                AudioCommand::ChangeMasterVolume(new_volume) => master_volume = new_volume,
                AudioCommand::ChangeTrackVolume(track_id, new_volume) => {
                    if let Some(track) = tracks.iter_mut().find(|t| t.data.id == track_id as u32) {
                        track.data.track_volume = new_volume;
                    }
                }
                AudioCommand::ToggleStep(pattern_id, track_id, step_idx) => {
                    if let Some(pattern) = patterns.iter_mut().find(|p| p.id == pattern_id) {
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
                        producer
                            .try_push(UiCommand::PatternLoaded(pattern.clone()))
                            .ok();
                    }
                }
                AudioCommand::DeleteTrack(track_id) => {
                    // remove all references of this track_id from saved patterns
                    let data_id = tracks[track_id].data.id;
                    for pattern in patterns.iter_mut() {
                        pattern.sequences.retain(|s| s.track_id != data_id);
                    }
                    events.retain(|e| {
                        if let crate::project::AudioBlockType::Sample(pid) = e.block_type {
                            pid != data_id as usize
                        } else {
                            true
                        }
                    });
                    tracks.remove(track_id);
                    producer.try_push(UiCommand::TrackDeleted(data_id)).ok();
                }

                AudioCommand::LoadTrack(mut track_data, samples) => {
                    // used to be tracks.len()
                    track_data.id = tracks
                        .iter()
                        .map(|x| x.data.id)
                        .max()
                        .map(|m| m + 1)
                        .unwrap_or(0) as u32;
                    let track = Track::from_data(track_data, samples);
                    tracks.push(track.clone()); // ownership clone
                    producer.try_push(UiCommand::TrackLoaded(track)).ok();
                }
                AudioCommand::ChangeBpm(new_bpm) => {
                    bpm = new_bpm;
                    producer.try_push(UiCommand::BpmChanged(new_bpm)).ok();
                }
                AudioCommand::ToggleTrackMute(track_id) => {
                    if let Some(track) = tracks.iter_mut().find(|t| t.data.id == track_id as u32) {
                        track.mute();
                    }
                }
                AudioCommand::TogglePlay => is_playing = !is_playing,
                AudioCommand::SaveProject => {
                    let project = Project::new(
                        name.clone(),
                        bpm,
                        master_volume,
                        &tracks,
                        patterns.clone(),
                        events.clone(),
                    );
                    project.save_to_toml(&project_path);
                    producer.try_push(UiCommand::SaveComplete).ok();
                    println!("saved to {}", &project_path);
                }
                AudioCommand::Shutdown => {
                    let project = Project::new(
                        name.clone(),
                        bpm,
                        master_volume,
                        &tracks,
                        patterns.clone(),
                        events.clone(),
                    );
                    project.save_to_toml(&project_path);

                    // save is complete
                    producer.try_push(UiCommand::SaveComplete).ok();
                    if !is_playing {
                        producer.try_push(UiCommand::ShutdownComplete).ok();
                    }
                    is_shutting_down = true;
                }
            }
        }

        // for each sample requested, mix in the appropriate track samples
        for sample in data.chunks_mut(2) {
            // fade audio off during app shutdown
            if is_shutting_down {
                shutdown_volume -= 0.0001;
                if shutdown_volume <= 0.0 {
                    producer.try_push(UiCommand::ShutdownComplete).ok();
                }
            }

            // Zero out the sample. Fill it if the song currently is_playing.
            sample[0] = 0.0; // left channel
            sample[1] = 0.0; // right channel

            if is_playing {
                // for each non-muted track currently playing in the song...
                for track in &mut tracks {
                    if !track.data.is_muted && track.is_playing {
                        // if the sample has fully played, mark it as not playing anymore
                        let pos = (track.position as usize) & !1; // align to stereo pair (even index)
                        let frac = track.position - track.position.floor();

                        if pos + 3 >= track.samples.len() {
                            track.is_playing = false;
                        } else {
                            track.current_volume = glacier_dsp::smooth_toward(
                                track.current_volume,
                                track.data.target_volume,
                                0.01,
                            );

                            // interpolate between current and next stereo pair
                            let l = track.samples[pos]
                                + frac * (track.samples[pos + 2] - track.samples[pos]);
                            let r = track.samples[pos + 1]
                                + frac * (track.samples[pos + 3] - track.samples[pos + 1]);

                            let gain = track.current_volume
                                * track.data.track_volume
                                * shutdown_volume
                                * master_volume;
                            sample[0] += l * gain;
                            sample[1] += r * gain;

                            track.position += 2.0 * track.playback_rate;

                            track.rms_l = glacier_dsp::smooth_toward(track.rms_l, l * l, 0.01);
                            track.rms_r = glacier_dsp::smooth_toward(track.rms_r, r * r, 0.01);
                            track.peak_hold = track.peak_hold.max(l.abs().max(r.abs()));
                        }
                    }
                }
            }
            // preview playback
            let pos = (preview_position as usize) & !1;
            if pos + 3 < preview_samples.len() {
                let frac = preview_position - preview_position.floor();
                let l =
                    preview_samples[pos] + frac * (preview_samples[pos + 2] - preview_samples[pos]);
                let r = preview_samples[pos + 1]
                    + frac * (preview_samples[pos + 3] - preview_samples[pos + 1]);
                sample[0] += l * master_volume;
                sample[1] += r * master_volume;
                preview_position += 2.0;
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
            producer
                .try_push(UiCommand::MasterLevel(
                    master_rms_l.sqrt(),
                    master_rms_r.sqrt(),
                    master_peak,
                ))
                .ok();
            master_peak = 0.0;
            for track in &mut tracks {
                producer
                    .try_push(UiCommand::TrackLevel(
                        track.data.id,
                        track.rms_l.sqrt(),
                        track.rms_r.sqrt(),
                        track.peak_hold,
                    ))
                    .ok();
                track.peak_hold = 0.0;
            }
        }

        // when the spectrum buffer is full, push fourier compute to ui
        if spectrum_buffer.len() >= SPECTRUM_WINDOW {
            let window = glacier_dsp::hann_window(SPECTRUM_WINDOW);
            let compensation = glacier_dsp::window_compensation(&window);

            let windowed: Vec<f32> = spectrum_buffer
                .iter()
                .zip(window.iter())
                .map(|(x, w)| x * w)
                .collect();

            let magnitudes = glacier_dsp::dft_window(&windowed);
            let db: Vec<f32> = magnitudes
                .iter()
                .map(|m| glacier_dsp::magnitude_to_db(*m, SPECTRUM_WINDOW, compensation))
                .collect();

            producer.try_push(UiCommand::SpectrumFrame(db)).ok();
            spectrum_buffer.clear();
        }

        if is_playing {
            sample_counter += data.len() as f32 / 2.0; // increment sample counter by number of samples requested : keep track of sample position

            // get amount of samples per step
            let samples_per_step = glacier_dsp::samples_per_step(config.sample_rate as f32, bpm);

            // update UI time
            let beat = current_step as f32 + (sample_counter / samples_per_step);
            producer.try_push(UiCommand::PlayheadPosition(beat)).ok();

            // increment the step if enough samples have passed
            if sample_counter >= samples_per_step {
                sample_counter = 0.0;

                let total_steps = events
                    .iter()
                    .map(|e| e.start_step + e.length)
                    .max()
                    .unwrap_or(16) as usize;
                current_step = (current_step + 1) % total_steps;

                producer
                    .try_push(UiCommand::StepAdvanced(current_step))
                    .ok();

                // build out each note
                let triggers: Vec<(usize, f32, u8)> = events
                    .iter()
                    .filter_map(|e| {
                        if let AudioBlockType::Pattern(pattern_id) = e.block_type {
                            if current_step >= e.start_step as usize
                                && current_step < (e.start_step + e.length) as usize
                            {
                                let local_step = current_step - e.start_step as usize;
                                return patterns
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
                                (s.track_id as usize, note.velocity, note.pitch)
                            })
                    })
                    .collect();

                for event in &events {
                    if let AudioBlockType::Sample(track_id) = event.block_type {
                        if current_step == event.start_step as usize {
                            if let Some(track) =
                                tracks.iter_mut().find(|t| t.data.id as usize == track_id)
                            {
                                track.position = 0.0;
                                track.is_playing = true;
                                track.data.target_volume = 1.0;
                                track.playback_rate = 1.0;
                            }
                        }
                    }
                }

                for (track_id, velocity, pitch) in triggers {
                    if let Some(track) = tracks
                        .iter_mut()
                        .find(|track| track.data.id as usize == track_id)
                    {
                        track.position = 0.0;
                        track.is_playing = true; // step function
                        track.data.target_volume = velocity / 127.0;
                        track.playback_rate =
                            glacier_dsp::semitones_to_rate(pitch, track.data.root_note)
                    }
                }
            }
        }
    };

    // attempt to create an output stream with device config
    let stream = match sample_format {
        SampleFormat::F32 => device.build_output_stream(&config, sequencer_callback, err_fn, None),
        sample_format => panic!("Unsupported sample format '{sample_format}'"),
    }
    .expect("Failed to build the output stream.");

    // start the output stream and return it
    stream.play().expect("Failed to play the output stream.");
    stream
}
