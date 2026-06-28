// audio.rs -  audio engine for sequencing compositions and applying DSP
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

// commands retrieved from the user interface
pub enum AudioCommand {
    // composition details
    ToggleStep(usize, usize, usize),
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
    ShutDown,
    SaveProject,
    SetProjectPath(String),

    // patterns
    DuplicatePattern(usize),
    AddPattern,
    DeletePattern(usize),

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

    // setup Tracks
    let mut tracks: Vec<Track> = get_tracks(&project);
    for track in tracks.iter() {
        producer.try_push(UiCommand::LoadTrack(track.clone())).ok();
    }

    // setup Patterns
    let mut patterns = project.patterns;
    for pattern in &patterns {
        producer
            .try_push(UiCommand::LoadPattern(pattern.clone()))
            .ok();
    }

    // setup Events
    let mut events = project.events;
    for event in &events {
        producer.try_push(UiCommand::LoadEvent(event.clone())).ok();
    }

    producer
        .try_push(UiCommand::LoadProjectPath(project_path.clone()))
        .ok();

    // setup bpm and volume
    let mut bpm: f32 = project.bpm;
    producer.try_push(UiCommand::LoadBpm(bpm)).ok();

    let mut master_volume = project.master_volume;
    producer
        .try_push(UiCommand::LoadMasterVolume(project.master_volume))
        .ok();

    // setup song state
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

    let mut preview_samples: Vec<f32> = Vec::new();
    let mut preview_position: f32 = 0.0;

    // audio callback
    // fills samples requested from CPAL audio driver
    let sequencer_callback = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        // parse incoming UI commands before fulfilling data callback
        while let Some(cmd) = consumer.try_pop() {
            match cmd {
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
                        let new_pattern = pattern.duplicate(patterns.len());
                        patterns.push(new_pattern.clone());
                        producer.try_push(UiCommand::LoadPattern(new_pattern)).ok();
                    }
                }

                AudioCommand::ToggleNote(pattern_id, track_id, step_idx, pitch) => {
                    if let Some(seq) = patterns[pattern_id]
                        .sequences
                        .iter_mut()
                        .find(|s| s.track_id == track_id)
                    {
                        if step_idx >= seq.steps.len() {
                            seq.steps.resize(step_idx + 1, Note::default());
                        }
                        let note = &mut seq.steps[step_idx];
                        if note.velocity > 0.0 && note.pitch == pitch {
                            *note = Note::default();
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
                        let mut steps = vec![Note::default(); step_idx + 1];
                        steps[step_idx] = Note {
                            velocity: 95.0,
                            pitch,
                        };
                        patterns[pattern_id]
                            .sequences
                            .push(Sequence { track_id, steps });
                    }
                }
                AudioCommand::AddPattern => {
                    let name = format!("Pattern {}", patterns.len() + 1);
                    let sequences = tracks
                        .iter()
                        .map(|instr| Sequence {
                            track_id: instr.data.id,
                            steps: vec![Note::default(); 16],
                        })
                        .collect();
                    let p = PatternData {
                        id: patterns.len(),
                        name,
                        sequences,
                    };
                    patterns.push(p.clone());
                    producer.try_push(UiCommand::LoadPattern(p)).ok();
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
                    for (i, p) in patterns.iter_mut().enumerate() {
                        p.id = i;
                    }
                }
                AudioCommand::DeleteAudioBlock(audio_block_id) => {
                    events.retain(|e| e.id != audio_block_id);
                }
                AudioCommand::Stop => {
                    is_playing = false;
                    current_step = 0;
                }
                AudioCommand::CreateAudioBlock(track, start_step, length, block_type) => {
                    // add new event to playlist
                    events.push(AudioBlock {
                        id: events.len(),
                        track,
                        start_step,
                        length: length as u32,
                        block_type,
                    });
                }
                AudioCommand::ChangeMasterVolume(new_volume) => master_volume = new_volume,
                AudioCommand::ChangeTrackVolume(track_id, new_volume) => {
                    tracks[track_id].data.track_volume = new_volume;
                }
                AudioCommand::ToggleStep(pattern_id, track_idx, step_idx) => {
                    let track_id = tracks[track_idx].data.id;
                    if let Some(seq) = patterns[pattern_id]
                        .sequences
                        .iter_mut()
                        .find(|s| s.track_id == track_id)
                    {
                        if step_idx >= seq.steps.len() {
                            seq.steps.resize(step_idx + 1, Note::default());
                        }
                        seq.steps[step_idx] = if seq.steps[step_idx].velocity > 0.0 {
                            Note::default()
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
                            track_id: track_id,
                            steps: vec![Note::default(); step_idx + 1],
                        };
                        seq.steps[step_idx] = Note {
                            velocity: 95.0,
                            pitch: 60,
                        };
                        patterns[pattern_id].sequences.push(seq);
                    }
                }
                AudioCommand::DeleteTrack(track_id) => {
                    // remove all references of this track_id from saved patterns
                    let data_id = tracks[track_id].data.id;
                    for pattern in patterns.iter_mut() {
                        pattern.sequences.retain(|s| s.track_id != data_id);
                    }
                    tracks.remove(track_id);
                }

                AudioCommand::LoadTrack(mut track_data, samples) => {
                    track_data.id = tracks.len() as u32;
                    let track = Track::from_data(track_data, samples);
                    tracks.push(track.clone()); // ownership clone
                    producer.try_push(UiCommand::LoadTrack(track)).ok();
                }
                AudioCommand::ChangeBpm(new_bpm) => bpm = new_bpm,
                AudioCommand::ToggleTrackMute(track_id) => tracks[track_id].mute(),
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
                AudioCommand::ShutDown => {
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
