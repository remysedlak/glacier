use serde::{Deserialize, Serialize};

/// Project data stores song information
#[derive(Serialize, Deserialize, Clone)]
pub struct Project {
    pub name: String,
    pub bpm: f32,
    pub master_volume: f32,
    pub events: Vec<AudioBlock>,
    pub instruments: Vec<InstrumentData>,
    pub patterns: Vec<PatternData>,
}

impl Default for Project {
    fn default() -> Project {
        Project {
            name: "New Project".to_string(),
            bpm: 120.0,
            master_volume: 1.00,
            events: vec![],
            instruments: vec![],
            patterns: vec![PatternData {
                id: 0,
                name: "Pattern 1".to_string(),
                sequences: vec![],
            }],
        }
    }
}

// types of blocks to be placed
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "kind", content = "id")]
pub enum AudioBlockType {
    Sample(usize),
    Pattern(usize),
    Mixing, // later for automating audio
}

/// AudioBlocks are how patterns are timed within a playlist
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AudioBlock {
    pub id: usize,
    pub track: usize,
    pub start_step: u32,
    pub length: u32,
    pub block_type: AudioBlockType,
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
    pub playback_rate: f32,
}

/// Instrument metadata stored on disk
#[derive(Serialize, Deserialize, Clone)]
pub struct InstrumentData {
    pub id: u32,
    pub name: String,
    pub path: String,
    pub is_muted: bool,

    // volume ramping
    pub target_volume: f32,
    pub track_volume: f32,

    // default 60 - C5
    pub root_note: u8,
}

/// Patterns store a set of sequences
#[derive(Serialize, Deserialize, Clone)]
pub struct PatternData {
    pub id: usize,
    pub name: String,
    pub sequences: Vec<Sequence>,
}

// A sequencer is a grid of steps for each instrument in ONE pattern
// The sequencer has a row of Sequence's

/// One row of steps for an instrument in a pattern
#[derive(Serialize, Deserialize, Clone)]
pub struct Sequence {
    pub instrument_id: u32,
    pub steps: Vec<Note>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Note {
    pub velocity: f32, // 0.0 = off, >0.0 = on
    pub pitch: u8,     // midi note 0-127, 60 = middle C5
}
impl Default for Note {
    // default to off note at middle C
    fn default() -> Self {
        Note { velocity: 0.0, pitch: 60 }
    }
}

/// Load project details into memory from file path
pub fn get_project(file_path: &str) -> Option<Project> {
    let text = std::fs::read_to_string(file_path).ok()?;
    toml::from_str(&text).ok()
}

/// load list of instruments with their audio data from project details
pub fn get_instruments(project: &Project) -> Vec<Instrument> {
    project
        .instruments
        .iter()
        .map(|track| Instrument {
            samples: path_to_vector(&track.path),
            position: 0.0,
            data: track.clone(),
            is_playing: false,
            current_volume: 0.0,
            show_velocity: false,
            playback_rate: 1.0,
        })
        .collect()
}

/// Save the project details to a location on disk
pub fn save_project(project: &Project, file_path: &str) {
    // convert project data to TOML format
    let text = toml::to_string(&project).unwrap();

    // write toml data to project file
    std::fs::write(file_path, text).unwrap();
}

/// load an instrument's float data from it's file path
pub fn path_to_vector(instrument_path: &str) -> Vec<f32> {
    let mut reader = match hound::WavReader::open(instrument_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to load {}: {}", instrument_path, e);
            return Vec::new();
        }
    };
    let spec = reader.spec();
    let divisor = 1 << (spec.bits_per_sample - 1);
    reader
        .samples::<i32>()
        .filter_map(|s| s.ok())
        .map(|s| s as f32 / divisor as f32)
        .collect()
}
