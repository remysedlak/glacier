use crate::project::{get_project, get_tracks, save_project, AudioBlock, AudioBlockType, Note, PatternData, Project, Sequence, Track, TrackData};
use crate::UiCommand;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    {SampleFormat, Stream},
};
use ringbuf::{
    traits::{Consumer, Producer},
    HeapCons, HeapProd,
};

pub const DEFAULT_BPM: f32 = 120.0;

// commands retrieved from the user interface
pub enum AudioCommand {
    // composition details
    ToggleStep(usize, usize, usize),
    ToggleNote(usize, u32, usize, u8), // pattern_id, track_id, step_idx, pitch
    ChangeBpm(f32),
    DeleteAudioBlock(usize),
    CreateAudioBlock(usize, u32, usize, AudioBlockType),

    // mixing
    ChangeMasterVolume(f32),
    ToggleTrackMute(usize),
    ChangeTrackVolume(usize, f32),

    // control
    TogglePlay,
    Stop,

    // project state
    ShutDown,
    SaveProject,

    // patterns
    DuplicatePattern(usize),
    AddPattern,
    DeletePattern(usize),

    // tracks
    LoadTrack(TrackData, Vec<f32>),
    DeleteTrack(usize),
}

/// initialize the CPAL engine with project file data and return the audio stream
pub fn init(mut consumer: HeapCons<AudioCommand>, mut producer: HeapProd<UiCommand>, project_file: Option<String>) -> Stream {
    // error callback
    let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);

    // cpal setup -> host, device, config
    let host = cpal::default_host();
    let device = host.default_output_device().expect("no output device available");
    let supported_config = device.default_output_config().expect("error getting default config");
    let config = supported_config.config();
    let sample_format = supported_config.sample_format();
    let sample_rate_f: f32 = config.sample_rate as f32;

    // load project from file path
    let project = project_file.as_deref().and_then(get_project).unwrap_or_else(|| Project::default());

    let project_path = project_file.unwrap_or_else(|| "assets/projects/new_project.toml".to_string());

    // Tracks state
    let mut tracks: Vec<Track> = get_tracks(&project);
    for track in tracks.iter() {
        producer.try_push(UiCommand::LoadTrack(track.clone())).ok();
    }

    // patterns state
    let mut patterns = project.patterns;
    for pattern in &patterns {
        producer.try_push(UiCommand::LoadPattern(pattern.clone())).ok();
    }

    // events state
    let mut events = project.events;
    for event in &events {
        producer.try_push(UiCommand::LoadEvent(event.clone())).ok();
    }

    producer.try_push(UiCommand::LoadProjectPath(project_path.clone())).ok();

    // saved bpm
    let mut bpm: f32 = project.bpm;
    producer.try_push(UiCommand::LoadBpm(bpm)).ok();

    // master volume
    let mut master_volume = project.master_volume;
    producer.try_push(UiCommand::LoadMasterVolume(project.master_volume)).ok();

    // initalize state
    let mut current_step = events.iter().map(|e| e.start_step + e.length).max().unwrap_or(16) as usize - 1;
    let mut is_playing = false;
    let mut is_shutting_down = false;
    let mut shutdown_volume: f32 = 1.00;
    let mut sample_counter: f32 = 0.0; // tracks how many samples passed, to track when a step passes
    let name = project.name.clone();

    // audio callback to fill samples requested from CPAL
    let sequencer_callback = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        // parse incoming UI commands before fulfilling data callback
        while let Some(cmd) = consumer.try_pop() {
            match cmd {
                AudioCommand::DuplicatePattern(pattern_id) => {
                    if let Some(pattern) = patterns.iter().find(|p| p.id == pattern_id).cloned() {
                        let mut new_pattern = pattern.clone();
                        new_pattern.id = patterns.len();
                        new_pattern.name = format!("{} Copy", pattern.name);
                        patterns.push(new_pattern.clone());
                        producer.try_push(UiCommand::LoadPattern(new_pattern)).ok();
                    }
                }

                AudioCommand::ToggleNote(pattern_id, track_id, step_idx, pitch) => {
                    if let Some(seq) = patterns[pattern_id].sequences.iter_mut().find(|s| s.track_id == track_id) {
                        let note = &mut seq.steps[step_idx];
                        if note.velocity > 0.0 && note.pitch == pitch {
                            *note = Note::default();
                        } else {
                            *note = Note { velocity: 95.0, pitch };
                        }
                    } else {
                        let mut steps = vec![Note::default(); 32];
                        steps[step_idx] = Note { velocity: 95.0, pitch };
                        patterns[pattern_id].sequences.push(Sequence { track_id, steps });
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
                AudioCommand::DeletePattern(id) => {
                    // remove the pattern from list of patterns
                    patterns.retain(|p| p.id != id);
                    // remove the pattern from list of events
                    events.retain(|e| {
                        if let AudioBlockType::Pattern(pid) = e.block_type {
                            pid != id
                        } else {
                            true
                        }
                    });
                    for (i, p) in patterns.iter_mut().enumerate() {
                        p.id = i;
                    }
                }
                AudioCommand::DeleteAudioBlock(id) => {
                    events.retain(|e| e.id != id);
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
                AudioCommand::ChangeMasterVolume(volume) => {
                    master_volume = volume;
                }
                AudioCommand::ChangeTrackVolume(i, vol) => {
                    tracks[i].data.track_volume = vol;
                }
                AudioCommand::ToggleStep(pattern_id, track_idx, step_idx) => {
                    let track_id = tracks[track_idx].data.id;
                    if let Some(seq) = patterns[pattern_id].sequences.iter_mut().find(|s| s.track_id == track_id) {
                        seq.steps[step_idx] = if seq.steps[step_idx].velocity > 0.0 {
                            Note::default()
                        } else {
                            Note { velocity: 95.0, pitch: 60 }
                        };
                    } else {
                        let mut seq = Sequence {
                            track_id: track_idx as u32,
                            steps: vec![Note::default(); 32],
                        };
                        seq.steps[step_idx] = Note { velocity: 95.0, pitch: 60 };
                        patterns[pattern_id].sequences.push(seq);
                    }
                }
                AudioCommand::DeleteTrack(i) => {
                    tracks.remove(i);
                }

                AudioCommand::LoadTrack(mut track_data, samples) => {
                    track_data.id = tracks.len() as u32;
                    let track = Track {
                        samples,
                        position: 0.0,
                        data: track_data,
                        is_playing: false,
                        current_volume: 0.0,
                        show_velocity: false,
                        playback_rate: 1.0,
                    };
                    tracks.push(track.clone());
                    producer.try_push(UiCommand::LoadTrack(track)).ok();
                }
                AudioCommand::ChangeBpm(new_bpm) => {
                    bpm = new_bpm;
                }
                AudioCommand::ToggleTrackMute(i) => {
                    tracks[i].data.is_muted = !tracks[i].data.is_muted;
                    tracks[i].position = 0.0;
                    tracks[i].is_playing = false;
                }
                AudioCommand::TogglePlay => {
                    // change state from pause to play, or play to pause
                    is_playing = !is_playing;
                }
                AudioCommand::SaveProject => {
                    // everything from current state
                    let project = Project {
                        name: name.clone(),
                        bpm,
                        master_volume,
                        tracks: tracks.iter().map(|i| i.data.clone()).collect(),
                        patterns: patterns.clone(),
                        events: events.clone(),
                    };
                    save_project(&project, &project_path);

                    // tell ui that we saved the audio finished up
                    producer.try_push(UiCommand::SaveComplete).ok();
                    println!("saved to {}", &project_path);
                }
                AudioCommand::ShutDown => {
                    let project = Project {
                        name: name.clone(),
                        bpm,
                        master_volume,
                        tracks: tracks.iter().map(|i| i.data.clone()).collect(),
                        patterns: patterns.clone(),
                        events: events.clone(),
                    };
                    save_project(&project, &project_path);

                    // save is complete
                    producer.try_push(UiCommand::SaveComplete).ok();
                    if !is_playing {
                        producer.try_push(UiCommand::ShutdownComplete).ok();
                    }
                    is_shutting_down = true;
                }
            }
        } // finish matching of commands sent from the UI

        // for each sample requested, mix in the appropriate track samples
        for sample in data.chunks_mut(2) {
            sample[0] = 0.0; // left channel
            sample[1] = 0.0; // right channel

            // fade out volume on shutdown
            if is_shutting_down {
                shutdown_volume -= 0.0001;
                if shutdown_volume <= 0.0 {
                    producer.try_push(UiCommand::ShutdownComplete).ok();
                }
            }

            // return audio data only when the song is actively playing
            if is_playing {
                for track in &mut tracks {
                    // ignore muted tracks
                    if !track.data.is_muted && track.is_playing {
                        // if the sample has fully played, mark it as not playing anymore
                        let pos = track.position as usize;
                        if pos + 1 >= track.samples.len() {
                            track.is_playing = false;
                        } else {
                            // volume ramping
                            if track.current_volume != track.data.target_volume {
                                let difference = track.data.target_volume - track.current_volume;
                                track.current_volume += difference * 0.01;
                            }
                            sample[0] += track.samples[pos] * track.current_volume * track.data.track_volume * shutdown_volume * master_volume;
                            sample[1] += track.samples[pos + 1] * track.current_volume * track.data.track_volume * shutdown_volume * master_volume;
                            track.position += 2.0 * track.playback_rate;
                        }
                    }
                }
            }
        }

        if is_playing {
            sample_counter += data.len() as f32 / 2.0; // increment sample counter by number of samples requested : keep track of sample position

            // get amount of samples per step
            let samples_per_step = sample_rate_f / (bpm / 60.0 * 4.0);

            // increment the step if enough samples have passed
            if sample_counter >= samples_per_step {
                sample_counter = 0.0;

                let total_steps = events.iter().map(|e| e.start_step + e.length).max().unwrap_or(16) as usize;
                current_step = (current_step + 1) % total_steps;

                producer.try_push(UiCommand::StepAdvanced(current_step)).ok();

                // build out each note
                let triggers: Vec<(usize, f32, u8)> = events
                    .iter()
                    .filter_map(|e| {
                        if let AudioBlockType::Pattern(pid) = e.block_type {
                            if current_step >= e.start_step as usize && current_step < (e.start_step + e.length) as usize {
                                let local_step = current_step - e.start_step as usize;
                                return patterns.iter().find(|p| p.id == pid).map(|p| (p, local_step));
                            }
                        }
                        None
                    })
                    .flat_map(|(p, local_step)| {
                        p.sequences
                            .iter()
                            .filter(move |s| local_step < s.steps.len() && s.steps[local_step].velocity > 0.0)
                            .map(move |s| {
                                let note = &s.steps[local_step];
                                (s.track_id as usize, note.velocity, note.pitch)
                            })
                    })
                    .collect();

                for (id, vel, pitch) in triggers {
                    if let Some(inst) = tracks.iter_mut().find(|i| i.data.id as usize == id) {
                        inst.position = 0.0;
                        inst.is_playing = true;
                        inst.data.target_volume = vel / 127.0;
                        let semitones = pitch as f32 - inst.data.root_note as f32;
                        inst.playback_rate = 2.0_f32.powf(semitones / 12.0);
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
