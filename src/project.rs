use serde::{Deserialize, Serialize};

/// Project data stores song information
#[derive(Serialize, Deserialize, Clone)]
pub struct ProjectFile {
    pub project_name: String,
    pub bpm: f32,
    // tracks: Vec<TrackData>,
    pub events: Vec<AudioBlock>,
    pub instruments: Vec<InstrumentData>,
    pub patterns: Vec<PatternData>,
}

// types of blocks to be placed
#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "kind", content = "id")]
pub enum AudioBlockType {
    Sample(usize),
    Pattern(usize),
    Mixing, // later for automating audio
}

/// AudioBlocks are how patterns are timed within a playlist
#[derive(Serialize, Deserialize, Clone)]
pub struct AudioBlock {
    pub start_step: u32,
    pub length: u32,
    pub block_type: AudioBlockType,
}

/// Patterns store a set of sequences
#[derive(Serialize, Deserialize, Clone)]
pub struct PatternData {
    pub id: usize,
    pub name: String,
    pub sequences: Vec<Sequence>,
}

/// Instrument data used at runtime for sound
#[derive(Clone)]
pub struct Instrument {
    pub data: InstrumentData,
    pub samples: Vec<f32>, // loaded from hound
    pub position: f32,
    pub is_playing: bool,
    pub current_volume: f32,
    pub show_velocity: bool,
}

/// Instrument metadata stored on disk
#[derive(Serialize, Deserialize, Clone)]
pub struct InstrumentData {
    pub id: u32,
    pub path: String, // file path
    pub name: String,
    pub is_muted: bool,
    // audio ramping
    pub target_volume: f32, // volume changes from sequences
    pub track_volume: f32,  // volume changes from volume knob
}

/// One row of steps for an instrument in a pattern
#[derive(Serialize, Deserialize, Clone)]
pub struct Sequence {
    pub instrument_id: u32,
    pub steps: Vec<f32>,
}

/// Load project details into memory from file path
pub fn get_project(project_file: &String) -> ProjectFile {
    let text: String = std::fs::read_to_string(&project_file).unwrap();
    let project: ProjectFile = toml::from_str(&text).unwrap();
    project
}

/// Create instrument's from convering saved metadata
pub fn get_instruments(project: &ProjectFile) -> Vec<Instrument> {
    let mut instruments: Vec<Instrument> = Vec::new();
    for track in &project.instruments {
        instruments.push(Instrument {
            samples: path_to_vector(&track.path),
            position: 0.0,
            data: track.clone(),
            is_playing: false,
            current_volume: 0.0,
            show_velocity: false,
        });
    }
    instruments
}

/// Save the project details to a location on disk
pub fn save_project(project: ProjectFile, project_file: String) {
    let text = toml::to_string(&project).unwrap();
    std::fs::write(project_file.clone(), text).unwrap();
}

/// load an instrument's float data from it's file path
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
