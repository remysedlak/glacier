use crate::UiCommand;
use cpal::traits::StreamTrait;
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{SampleFormat, Stream};
use ringbuf::traits::{Consumer, Producer};
use ringbuf::{HeapCons, HeapProd};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct ProjectFile {
    project_name: String,
    bpm: f32,
    tracks: Vec<TrackData>,
}

#[derive(Serialize, Deserialize)]
struct TrackData {
    name: String,
    mute: bool,
    steps: Vec<f32>,
    sample_path: String,
    position: f32,
    target_volume: f32,
    is_playing: bool,
}

pub enum AudioCommand {
    ToggleStep(usize, usize),
    ChangeBpm(f32),
    ToggleMute(usize),
    TogglePlay,
    ShutDown,
}

// instrument struct: track information about ONE instrument
struct Instrument {
    samples: Vec<f32>, // the literal raw buffer of audio
    position: f32,     // current playback position
    steps: Vec<f32>,   // the sequence of steps to play back
    is_playing: bool,
    name: String,
    mute: bool,
    path: String,
    // audio ramping
    target_volume: f32,
    current_volume: f32,
}

pub fn init(mut consumer: HeapCons<AudioCommand>, mut producer: HeapProd<UiCommand>) -> Stream {
    println!("STARTING REMY'S AUDIO ENGINE");

    let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);

    // use the default host to find devices
    let host = cpal::default_host();

    // access the devices data streams
    let device = host
        .default_output_device()
        .expect("no output device available");

    // a config must be defined to use the device properlyz
    let supported_config = device
        .default_output_config()
        .expect("error getting default config");

    let config = supported_config.config();
    let sample_format = supported_config.sample_format();

    let sample_ratef: f32 = config.sample_rate as f32;

    // load raw string data from toml file to handle init.
    let text: String = std::fs::read_to_string("my_song.toml").unwrap();
    let project: ProjectFile = toml::from_str(&text).unwrap();

    let mut bpm: f32 = project.bpm;
    let mut sample_counter: f32 = 0.0; // tracks how many samples passed, to track when a step passes
    let mut current_step = 15;

    // user hardware specific
    println!("SAMPLE RATE: {}", config.sample_rate);

    // load a set of instruments to play
    let mut instruments: Vec<Instrument> = Vec::new();
    for track in project.tracks {
        instruments.push(Instrument {
            samples: path_to_vector(&track.sample_path),
            position: track.position,
            name: track.name,
            steps: track.steps,
            is_playing: false,
            target_volume: track.target_volume,
            current_volume: 0.0,
            mute: track.mute,
            path: track.sample_path,
        })
    }

    // load the stored BPM onto the UI screen
    producer.try_push(UiCommand::LoadBpm(bpm)).ok();

    // load each instrument individually to the UI screen
    for (i, instrument) in instruments.iter().enumerate() {
        let bools: Vec<bool> = instrument.steps.iter().map(|step| *step > 0.0).collect();
        producer
            .try_push(UiCommand::LoadTrack(
                i,
                instrument.name.clone(),
                bools.try_into().unwrap(),
                instrument.mute,
            ))
            .ok();
    }

    let mut is_playing = false;
    let mut is_shutting_down = false;
    let mut shutdown_volume: f32 = 1.00; // multiplied by output data. only affects when decremented slowly to fade out audio on exit

    // audio callback to fill samples requested from CPAL
    let sequencer_callback = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        // before performing an audio callback, check if UI pushed any commands to pop
        while let Some(cmd) = consumer.try_pop() {
            match cmd {
                AudioCommand::ToggleStep(x, y) => {
                    if instruments[x].steps[y] > 0.0 {
                        instruments[x].steps[y] = 0.0;
                    } else {
                        instruments[x].steps[y] = 95.0;
                    }
                }
                AudioCommand::ChangeBpm(new_bpm) => {
                    bpm = new_bpm;
                }
                AudioCommand::ToggleMute(i) => {
                    instruments[i].mute = !instruments[i].mute;
                    instruments[i].position = 0.0;
                    instruments[i].is_playing = false;
                }
                AudioCommand::TogglePlay => {
                    is_playing = !is_playing;
                }
                AudioCommand::ShutDown => {
                    let project = ProjectFile {
                        project_name: "My Song".to_string(),
                        bpm,
                        tracks: instruments
                            .iter()
                            .map(|inst| TrackData {
                                name: inst.name.clone(),
                                mute: inst.mute,
                                steps: inst.steps.clone(),
                                sample_path: inst.path.clone(),
                                position: 0.0,
                                target_volume: inst.target_volume,
                                is_playing: false,
                            })
                            .collect(),
                    };
                    let text = toml::to_string(&project).unwrap();
                    std::fs::write("my_song.toml", text).unwrap();

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
                    if !instrument.mute && instrument.is_playing {
                        // if the sample fully played, mark as not playing anymore
                        if instrument.position >= instrument.samples.len() as f32 {
                            instrument.is_playing = false;
                        } else {
                            instrument.is_playing = true;

                            // volume ramping
                            if instrument.current_volume != instrument.target_volume {
                                let difference =
                                    instrument.target_volume - instrument.current_volume;
                                instrument.current_volume += difference * 0.01;
                            }

                            // add current samples to left and right channel and increment instruments position
                            sample[0] += instrument.samples[(instrument.position as f32) as usize]
                                * instrument.current_volume
                                * shutdown_volume;
                            sample[1] += instrument.samples
                                [(instrument.position as f32) as usize + 1]
                                * instrument.current_volume
                                * shutdown_volume;
                            instrument.position += 2.0;
                        }
                    }
                }
            }
        }

        if is_playing {
            sample_counter += data.len() as f32 / 2.0; // increment sample counter by number of samples requested : keep track of sample position

            // get amount of samples per step
            let samples_per_step = sample_ratef / (bpm / 60.0 * 4.0);

            // increment the step if enough samples have passed
            if sample_counter >= samples_per_step {
                sample_counter = 0.0;
                current_step = (current_step + 1) % 16;
                producer
                    .try_push(UiCommand::StepAdvanced(current_step))
                    .ok();
                // if the instrument plays on the newly incremented step, restart its position, enabling it for the next callback
                for instrument in &mut instruments {
                    if instrument.steps[current_step] > 0.0 {
                        instrument.position = 0.0;
                        instrument.is_playing = true;
                        instrument.target_volume = instrument.steps[current_step] / 127.0;
                    }
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

// load an instrument path into a vector of floats
pub fn path_to_vector(instrument_path: &str) -> Vec<f32> {
    // Open the WAV file using the hound library
    let mut reader = match hound::WavReader::open(instrument_path) {
        Ok(result) => result,
        Err(err) => panic!("{}", err),
    };

    // find out how many bits are in a sample to properly normalize values
    let spec = reader.spec();
    let divisor = 1 << (spec.bits_per_sample - 1);

    // Read all samples as i32 (32-bit audio)
    let samples = reader.samples::<i32>();

    // Convert i16 samples to f32 normalized values
    let vector: Vec<f32> = samples
        .map(|result| result.unwrap()) // Unwrap each Result<i32>
        .map(|i32_value| i32_value as f32 / divisor as f32) // Normalize to [-1.0, 1.0]
        .collect();
    vector
}
