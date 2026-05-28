use serde::{Deserialize, Serialize};

/// Project data stores song information
#[derive(Serialize, Deserialize, Clone)]
pub struct Project {
    pub name: String,
    pub bpm: f32,
    pub master_volume: f32,
    pub events: Vec<AudioBlock>,
    pub tracks: Vec<TrackData>,
    pub patterns: Vec<PatternData>,
}

impl Project {
    pub fn new(name: String, bpm: f32, master_volume: f32, tracks: &Vec<Track>, patterns: Vec<PatternData>, events: Vec<AudioBlock>) -> Project {
        Project {
            name: name.clone(),
            bpm,
            master_volume,
            tracks: tracks.iter().map(|track| track.data.clone()).collect(),
            patterns: patterns.clone(),
            events: events.clone(),
        }
    }
}

impl Default for Project {
    fn default() -> Project {
        Project {
            name: "New Project".to_string(),
            bpm: 120.0,
            master_volume: 1.00,
            events: vec![],
            tracks: vec![],
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

/// Track data used at runtime for sound
#[derive(Clone)]
pub struct Track {
    pub data: TrackData,
    pub samples: Vec<f32>, // loaded from hound
    pub position: f32,
    pub is_playing: bool,
    pub current_volume: f32,
    pub show_velocity: bool,
    pub playback_rate: f32,
}

impl Track {
    // build track with data at default states
    pub fn from_data(data: TrackData, samples: Vec<f32>) -> Track {
        Track {
            samples,
            position: 0.0,
            data,
            is_playing: false,
            current_volume: 0.0,
            show_velocity: false,
            playback_rate: 1.0,
        }
    }
}

/// Track  metadata stored on disk
#[derive(Serialize, Deserialize, Clone)]
pub struct TrackData {
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

// A sequencer is a grid of steps for each track in ONE pattern
// The sequencer has a row of Sequence's

/// One row of steps for an track in a pattern
#[derive(Serialize, Deserialize, Clone)]
pub struct Sequence {
    pub track_id: u32,
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

/// load list of tracks with their audio data from project details
pub fn get_tracks(project: &Project) -> Vec<Track> {
    project
        .tracks
        .iter()
        .map(|track| Track::from_data(track.clone(), path_to_vector(&track.path)))
        .collect()
}

/// Save the project details to a location on disk
pub fn save_project_to_file(project: &Project, file_path: &str) {
    // convert project data to TOML format
    let text = toml::to_string(&project).unwrap();

    // write toml data to project file
    std::fs::write(file_path, text).unwrap();
}

/// load a track's float data from it's file path
pub fn path_to_vector(track_path: &str) -> Vec<f32> {
    let mut reader = match hound::WavReader::open(track_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to load {}: {}", track_path, e);
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
