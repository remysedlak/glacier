use crate::project::{
    get_instruments, get_project, save_project, AudioBlock, AudioBlockType, Instrument, InstrumentData, Note, PatternData, Project, Sequence,
};
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
    ToggleNote(usize, u32, usize, u8), // pattern_id, instrument_id, step_idx, pitch
    ChangeBpm(f32),

    // mixing
    ChangeMasterVolume(f32),
    ToggleTrackMute(usize),

    // control
    TogglePlay,
    Stop,
    ShutDown,
    SaveProject,
    DuplicatePattern(usize),
    AddPattern,
    DeletePattern(usize),

    LoadInstrument(InstrumentData, Vec<f32>),

    DeleteTrack(usize),
    ChangeTrackVolume(usize, f32),
    DeleteAudioBlock(usize),
    CreateAudioBlock(usize, u32, usize, AudioBlockType),
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
    let project = project_file.as_deref().and_then(get_project).unwrap_or_else(|| Project {
        name: "New Project".to_string(),
        bpm: 120.0,
        events: vec![],
        instruments: vec![],
        patterns: vec![PatternData {
            id: 0,
            name: "Pattern 1".to_string(),
            sequences: vec![],
        }],
    });

    let project_file = project_file.unwrap_or_else(|| "assets/projects/new_project.toml".to_string());

    // load a set of instruments to play
    let mut instruments: Vec<Instrument> = get_instruments(&project);
    let mut patterns = project.patterns;
    for pattern in &patterns {
        producer.try_push(UiCommand::LoadPattern(pattern.clone())).ok();
    }
    let mut bpm: f32 = project.bpm;
    let mut events = project.events;
    for event in &events {
        producer.try_push(UiCommand::LoadEvent(event.clone())).ok();
    }

    producer.try_push(UiCommand::LoadProjectFile(project_file.clone())).ok();

    // wrap back on first hit (start on 0)
    let mut current_step = events.iter().map(|e| e.start_step + e.length).max().unwrap_or(16) as usize - 1;

    // load instruments, bpm, volume to UI
    producer.try_push(UiCommand::LoadBpm(bpm)).ok();
    producer.try_push(UiCommand::LoadMasterVolume(1.0)).ok();
    for instrument in instruments.iter() {
        producer.try_push(UiCommand::LoadInstrument(instrument.clone())).ok();
    }

    // initalize state
    let mut is_playing = false;
    let mut is_shutting_down = false;
    let mut shutdown_volume: f32 = 1.00;
    let mut sample_counter: f32 = 0.0; // tracks how many samples passed, to track when a step passes
    let mut master_volume = 1.0;
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

                AudioCommand::ToggleNote(pattern_id, instrument_id, step_idx, pitch) => {
                    if let Some(seq) = patterns[pattern_id].sequences.iter_mut().find(|s| s.instrument_id == instrument_id) {
                        let note = &mut seq.steps[step_idx];
                        if note.velocity > 0.0 && note.pitch == pitch {
                            *note = Note::default();
                        } else {
                            *note = Note { velocity: 95.0, pitch };
                        }
                    } else {
                        let mut steps = vec![Note::default(); 32];
                        steps[step_idx] = Note { velocity: 95.0, pitch };
                        patterns[pattern_id].sequences.push(Sequence { instrument_id, steps });
                    }
                }
                AudioCommand::AddPattern => {
                    let name = format!("Pattern {}", patterns.len() + 1);
                    let sequences = instruments
                        .iter()
                        .map(|instr| Sequence {
                            instrument_id: instr.data.id,
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
                    instruments[i].data.track_volume = vol;
                }
                AudioCommand::ToggleStep(pattern_id, instrument_idx, step_idx) => {
                    let instrument_id = instruments[instrument_idx].data.id;
                    if let Some(seq) = patterns[pattern_id].sequences.iter_mut().find(|s| s.instrument_id == instrument_id) {
                        seq.steps[step_idx] = if seq.steps[step_idx].velocity > 0.0 {
                            Note::default()
                        } else {
                            Note { velocity: 95.0, pitch: 60 }
                        };
                    } else {
                        let mut seq = Sequence {
                            instrument_id: instrument_idx as u32,
                            steps: vec![Note::default(); 32],
                        };
                        seq.steps[step_idx] = Note { velocity: 95.0, pitch: 60 };
                        patterns[pattern_id].sequences.push(seq);
                    }
                }
                AudioCommand::DeleteTrack(i) => {
                    instruments.remove(i);
                }

                AudioCommand::LoadInstrument(mut instrument_data, samples) => {
                    instrument_data.id = instruments.len() as u32;
                    let instrument = Instrument {
                        samples,
                        position: 0.0,
                        data: instrument_data,
                        is_playing: false,
                        current_volume: 0.0,
                        show_velocity: false,
                        playback_rate: 1.0,
                    };
                    instruments.push(instrument.clone());
                    producer.try_push(UiCommand::LoadInstrument(instrument)).ok();
                }
                AudioCommand::ChangeBpm(new_bpm) => {
                    bpm = new_bpm;
                }
                AudioCommand::ToggleTrackMute(i) => {
                    instruments[i].data.is_muted = !instruments[i].data.is_muted;
                    instruments[i].position = 0.0;
                    instruments[i].is_playing = false;
                }
                AudioCommand::TogglePlay => {
                    is_playing = !is_playing;
                }
                AudioCommand::SaveProject => {
                    let project = Project {
                        name: name.clone(),
                        bpm,
                        instruments: instruments.iter().map(|i| i.data.clone()).collect(),
                        patterns: patterns.clone(),
                        events: events.clone(),
                    };
                    save_project(&project, &project_file);
                    producer.try_push(UiCommand::SaveComplete).ok();
                    println!("saved to {}", project_file.clone());
                }
                AudioCommand::ShutDown => {
                    let project = Project {
                        name: name.clone(),
                        bpm,
                        instruments: instruments.iter().map(|i| i.data.clone()).collect(),
                        patterns: patterns.clone(),
                        events: events.clone(),
                    };
                    save_project(&project, &project_file);

                    // save is complete
                    producer.try_push(UiCommand::SaveComplete).ok();
                    if !is_playing {
                        producer.try_push(UiCommand::ShutdownComplete).ok();
                    }
                    is_shutting_down = true;
                }
            }
        } // finish matching of commands sent from the UI

        // for each sample requested, mix in the appropriate instrument samples
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
                for instrument in &mut instruments {
                    // ignore muted instruments
                    if !instrument.data.is_muted && instrument.is_playing {
                        // if the sample has fully played, mark it as not playing anymore
                        let pos = instrument.position as usize;
                        if pos + 1 >= instrument.samples.len() {
                            instrument.is_playing = false;
                        } else {
                            // volume ramping
                            if instrument.current_volume != instrument.data.target_volume {
                                let difference = instrument.data.target_volume - instrument.current_volume;
                                instrument.current_volume += difference * 0.01;
                            }
                            sample[0] +=
                                instrument.samples[pos] * instrument.current_volume * instrument.data.track_volume * shutdown_volume * master_volume;
                            sample[1] += instrument.samples[pos + 1]
                                * instrument.current_volume
                                * instrument.data.track_volume
                                * shutdown_volume
                                * master_volume;
                            instrument.position += 2.0 * instrument.playback_rate;
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

                // triggers now carry pitch too
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
                            .filter(move |s| s.steps[local_step % s.steps.len()].velocity > 0.0)
                            .map(move |s| {
                                let note = &s.steps[local_step % s.steps.len()];
                                (s.instrument_id as usize, note.velocity, note.pitch)
                            })
                    })
                    .collect();

                for (id, vel, pitch) in triggers {
                    if let Some(inst) = instruments.iter_mut().find(|i| i.data.id as usize == id) {
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
