use crate::UiCommand;
use cpal::traits::StreamTrait;
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{SampleFormat, Stream};
use ringbuf::traits::{Consumer, Producer};
use ringbuf::{HeapCons, HeapProd};

pub enum AudioCommand {
    ToggleStep(usize, usize),
    ChangeBpm(f32),
}

// instrument struct: track information about ONE instrument
struct Instrument {
    samples: Vec<f32>, // the literal raw buffer of audio
    position: f32,     // current playback position
    steps: Vec<f32>,   // the sequence of steps to play back
    is_playing: bool,
    name: String,

    // audio ramping
    target_volume: f32,
    current_volume: f32,
}

pub fn init(mut consumer: HeapCons<AudioCommand>, mut producer: HeapProd<UiCommand>) -> Stream {
    println!("STARTING REMY'S DAW");
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
    let mut bpm: f32 = 120.0;

    // track how many samples have passed since the last step
    let mut sample_counter: f32 = 0.0;
    let mut current_step = 0;

    println!("SAMPLE RATE: {}", config.sample_rate);

    // set of instruments to test
    let mut instruments: Vec<Instrument> = Vec::new();
    instruments.push(Instrument {
        samples: path_to_vector("AttackS.wav"),
        position: 0.0,
        name: "AttackS.wav".to_string(),
        steps: vec![
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ],
        is_playing: false,
        target_volume: 1.0,
        current_volume: 0.0,
    });
    instruments.push(Instrument {
        samples: path_to_vector("PopOHH.wav"),
        position: 0.0,
        name: "PopOHH.wav".to_string(),
        steps: vec![
            0.0, 0.0, 95.0, 0.0, 0.0, 0.0, 95.0, 0.0, 0.0, 0.0, 95.0, 0.0, 0.0, 0.0, 95.0, 0.0,
        ],
        is_playing: false,
        target_volume: 1.0,
        current_volume: 0.0,
    });
    instruments.push(Instrument {
        samples: path_to_vector("SharpK.wav"),
        position: 0.0,
        name: "SharpK.wav".to_string(),
        steps: vec![
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ],
        is_playing: false,
        target_volume: 1.0,
        current_volume: 0.0,
    });

    for (i, instrument) in instruments.iter().enumerate() {
        let bools: Vec<bool> = instrument.steps.iter().map(|step| *step > 0.0).collect();
        producer
            .try_push(UiCommand::LoadTrack(
                i,
                instrument.name.clone(),
                bools.try_into().unwrap(),
            ))
            .ok();
    }

    // audio callback to fill samples requested from CPAL
    let sequencer_callback = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        sample_counter += data.len() as f32 / 2.0; // increment sample counter by number of samples requested : keep track of sample position

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
            }
        }

        // get amount of samples per step
        let samples_per_step = sample_ratef / (bpm / 60.0 * 4.0);

        // for each sample requested, mix in the appropriate instrument samples
        for sample in data.chunks_mut(2) {
            sample[0] = 0.0; // left
            sample[1] = 0.0; // right

            for instrument in &mut instruments {
                // if the instrument is active at the current step, mix in its sample
                if instrument.is_playing {
                    if instrument.position >= instrument.samples.len() as f32 {
                        instrument.is_playing = false;
                    } else {
                        instrument.is_playing = true;
                        if instrument.current_volume != instrument.target_volume {
                            let difference = instrument.target_volume - instrument.current_volume;
                            instrument.current_volume += difference * 0.01;
                        }
                        sample[0] += instrument.samples[(instrument.position as f32) as usize]
                            * instrument.current_volume;
                        sample[1] += instrument.samples[(instrument.position as f32) as usize + 1]
                            * instrument.current_volume;
                        instrument.position += 2.0;
                    }
                }
            }
        }
        if sample_counter >= samples_per_step {
            sample_counter = 0.0;
            current_step = (current_step + 1) % 16;
            producer
                .try_push(UiCommand::StepAdvanced(current_step))
                .ok();
            for instrument in &mut instruments {
                if instrument.steps[current_step] > 0.0 {
                    instrument.position = 0.0;
                    instrument.is_playing = true;
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
