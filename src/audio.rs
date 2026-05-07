use crate::project::*;
use crate::UiCommand;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    {SampleFormat, Stream},
};
use ringbuf::{
    traits::{Consumer, Producer},
    {HeapCons, HeapProd},
};

// commands retrieved from the user interface
pub enum AudioCommand {
    ToggleStep(usize, usize, usize),
    ChangeBpm(f32),
    ChangeMasterVolume(f32),
    ToggleMute(usize),
    TogglePlay,
    ShutDown,
    SaveProject,
    AddInstrument(String),
    DeleteTrack(usize),
    ChangeTrackVolume(usize, f32),
    DeleteAudioBlock(usize),
    CreateAudioBlock(usize, u32, usize, AudioBlockType),
}

/// initialize the audio engine with cpal and data from a project file
pub fn init(mut consumer: HeapCons<AudioCommand>, mut producer: HeapProd<UiCommand>, project_file: String) -> Stream {
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
    let project = get_project(&project_file);
    dbg!(&project.project_name);

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
    let mut current_step = patterns
        .iter()
        .flat_map(|p| p.sequences.iter())
        .map(|s| s.steps.len())
        .max()
        .unwrap_or(16)
        - 1; // wrap back on first hit

    // load instruments, bpm, volume to UI
    producer.try_push(UiCommand::LoadBpm(bpm)).ok();
    producer.try_push(UiCommand::LoadMasterVolume(1.0)).ok();
    for instrument in instruments.iter() {
        producer.try_push(UiCommand::LoadInstrument(instrument.clone())).ok();
    }

    let mut is_playing = false;
    let mut is_shutting_down = false;
    let mut shutdown_volume: f32 = 1.00;
    let mut sample_counter: f32 = 0.0; // tracks how many samples passed, to track when a step passes
    let mut master_volume = 1.0;
    let project_name = project.project_name.clone();

    // audio callback to fill samples requested from CPAL
    let sequencer_callback = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        // parse incoming UI commands before fulfilling data callback
        while let Some(cmd) = consumer.try_pop() {
            match cmd {
                AudioCommand::DeleteAudioBlock(id) => {
                    events.retain(|e| e.id != id);
                }
                AudioCommand::CreateAudioBlock(track, start_step, length, block_type) => {
                    // add new event to playlist
                    events.push(AudioBlock {
                        id: events.len(),
                        track: track,
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
                        seq.steps[step_idx] = if seq.steps[step_idx] > 0.0 { 0.0 } else { 95.0 };
                    } else {
                        let mut seq = Sequence {
                            instrument_id: instrument_idx as u32,
                            steps: vec![0.0f32; 32],
                        };
                        seq.steps[step_idx] = 95.0;
                        patterns[pattern_id].sequences.push(seq);
                    }
                }
                AudioCommand::DeleteTrack(i) => {
                    instruments.remove(i);
                }
                AudioCommand::AddInstrument(path) => {
                    let file_name = std::path::Path::new(&path) // get the file name
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string();
                    let instrument = Instrument {
                        samples: path_to_vector(&path),
                        position: 0.0,
                        data: InstrumentData {
                            id: (instruments.len() - 1) as u32,
                            path: path.clone(),
                            track_volume: 1.0,
                            target_volume: 1.0,
                            is_muted: false,
                            name: file_name.clone(),
                        },
                        is_playing: false,
                        current_volume: 0.0,
                        show_velocity: false,
                    };
                    instruments.push(instrument.clone());
                    producer.try_push(UiCommand::LoadInstrument(instrument)).ok();
                }
                AudioCommand::ChangeBpm(new_bpm) => {
                    bpm = new_bpm;
                }
                AudioCommand::ToggleMute(i) => {
                    instruments[i].data.is_muted = !instruments[i].data.is_muted;
                    instruments[i].position = 0.0;
                    instruments[i].is_playing = false;
                }
                AudioCommand::TogglePlay => {
                    is_playing = !is_playing;
                }
                AudioCommand::SaveProject => {
                    let project = ProjectFile {
                        project_name: project_name.clone(),
                        bpm: bpm,
                        instruments: instruments.iter().map(|i| i.data.clone()).collect(),
                        patterns: patterns.clone(),
                        events: events.clone(),
                    };
                    save_project(project, project_file.clone());
                    producer.try_push(UiCommand::SaveComplete).ok();
                    println!("saved to {}", project_file.clone());
                }
                AudioCommand::ShutDown => {
                    let project = ProjectFile {
                        project_name: project_name.clone(),
                        bpm,
                        instruments: instruments.iter().map(|i| i.data.clone()).collect(),
                        patterns: patterns.clone(),
                        events: events.clone(),
                    };
                    save_project(project, project_file.clone());

                    // save is complete
                    producer.try_push(UiCommand::SaveComplete).ok();
                    if !is_playing {
                        producer.try_push(UiCommand::ShutdownComplete).ok();
                    }
                    is_shutting_down = true;
                }
            }
        }
        // UI commands are over; deal with the audio samples requested
        //
        //

        // for each sample requested, mix in the appropriate instrument samples
        for sample in data.chunks_mut(2) {
            sample[0] = 0.0; // left channel
            sample[1] = 0.0; // right channel

            // for each sample, decrement the shutdown volume slowly
            if is_shutting_down {
                shutdown_volume -= 0.0001;
                if shutdown_volume <= 0.0 {
                    producer.try_push(UiCommand::ShutdownComplete).ok();
                }
            }

            // only if the audio is not currently paused
            if is_playing {
                for instrument in &mut instruments {
                    // ignore muted instruments
                    if !instrument.data.is_muted && instrument.is_playing {
                        // if the sample fully played, mark as not playing anymore
                        if instrument.position >= instrument.samples.len() as f32 {
                            instrument.is_playing = false;
                        } else {
                            instrument.is_playing = true;

                            // volume ramping
                            if instrument.current_volume != instrument.data.target_volume {
                                let difference = instrument.data.target_volume - instrument.current_volume;
                                instrument.current_volume += difference * 0.01;
                            }

                            // add current samples to left and right channel and increment instruments position
                            sample[0] += instrument.samples[(instrument.position as f32) as usize]
                                * instrument.current_volume
                                * instrument.data.track_volume
                                * shutdown_volume
                                * master_volume;
                            sample[1] += instrument.samples[(instrument.position as f32) as usize + 1]
                                * instrument.current_volume
                                * instrument.data.track_volume
                                * shutdown_volume
                                * master_volume;
                            instrument.position += 2.0;
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

                // current step follows the longest instrument track
                current_step = (current_step + 1)
                    % patterns
                        .iter()
                        .flat_map(|p| p.sequences.iter())
                        .map(|s| s.steps.len())
                        .max()
                        .unwrap_or(16);

                producer.try_push(UiCommand::StepAdvanced(current_step)).ok();

                let triggers: Vec<(usize, f32)> = events
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
                            .filter(move |s| s.steps[local_step % s.steps.len()] > 0.0)
                            .map(move |s| (s.instrument_id as usize, s.steps[local_step % s.steps.len()]))
                    })
                    .collect();

                for (id, vel) in triggers {
                    instruments[id].position = 0.0;
                    instruments[id].is_playing = true;
                    instruments[id].data.target_volume = vel / 127.0;
                }
            }
        }
    };

    let stream = match sample_format {
        SampleFormat::F32 => device.build_output_stream(&config, sequencer_callback, err_fn, None),
        sample_format => panic!("Unsupported sample format '{sample_format}'"),
    }
    .expect("Failed to build the output stream.");

    // start the output stream
    stream.play().expect("Failed to play the output stream.");
    stream
}
