// project.rs - structured objects to store song data
use serde::{Deserialize, Serialize};

/// Project data stores song information
#[derive(Serialize, Deserialize, Clone)]
pub struct Project {
    pub name: String, // Name of the project
    pub bpm: f32,     // beats per minute
    pub master_volume: f32,
    pub events: Vec<AudioBlock>,    // Instrument + time +  location
    pub tracks: Vec<TrackData>,     // List of instruments
    pub patterns: Vec<PatternData>, // List of patterns
}

impl Project {
    /// Create a new project
    pub fn new(
        name: String,
        bpm: f32,
        master_volume: f32,
        tracks: &[Track],
        patterns: Vec<PatternData>,
        events: Vec<AudioBlock>,
    ) -> Project {
        Project {
            name: name.clone(),
            bpm,
            master_volume,
            tracks: tracks.iter().map(|track| track.data.clone()).collect(),
            patterns: patterns.clone(),
            events: events.clone(),
        }
    }
    /// Save the project details to a location on disk
    pub fn save_to_toml(&self, file_path: &str) {
        let path = std::path::Path::new(file_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let text = toml::to_string(self).unwrap();
        if let Err(e) = std::fs::write(file_path, &text) {
            eprintln!("Failed to save project: {}", e);
        }
    }
    /// Use the default DAW file
    pub fn default_project_file() -> String {
        "assets/projects/new_project.toml".to_string()
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
    Sample(usize),  // Instrument
    Pattern(usize), // Pattern
    Mixing,         // Automation
}

/// AudioBlocks are how audio events are timed within a playlist
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AudioBlock {
    pub id: usize,                  // uuid
    pub track: usize,               // track id
    pub start_step: u32,            // what step does this patterns start at?
    pub length: u32,                // how long is this event? (cut/extended?)
    pub block_type: AudioBlockType, // pattern/instrument/mixing
}

/// Runtime Track object
#[derive(Clone)]
pub struct Track {
    pub data: TrackData,
    pub samples: Vec<f32>,   // raw float values
    pub is_playing: bool,    //  based on position of song
    pub show_velocity: bool, // sequencer.rs ui

    // dsp runtime
    pub rms_l: f32,
    pub rms_r: f32,
    pub peak_hold: f32,
    pub position: f32,
    pub playback_rate: f32,
    pub current_volume: f32, // track loudness
}

impl Track {
    // build track with data at default states
    pub fn from_data(data: TrackData, samples: Vec<f32>) -> Track {
        Track {
            samples,
            data,
            is_playing: false,
            current_volume: 0.0,
            show_velocity: false,
            // default dsp
            position: 0.0,
            playback_rate: 1.0,
            rms_l: 0.0,
            rms_r: 0.0,
            peak_hold: 0.0,
        }
    }
    pub fn mute(&mut self) {
        self.data.is_muted = !self.data.is_muted;
        self.position = 0.0;
        self.is_playing = false;
    }
}

/// Track metadata stored on disk
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

impl PatternData {
    pub fn duplicate(&self, id: usize) -> PatternData {
        let mut new_pattern = self.clone();
        new_pattern.id = id;
        new_pattern.name = format!("{} Copy", self.name);
        new_pattern
    }
}

// A sequencer is a grid of steps for each track in ONE pattern
// The sequencer has a row of Sequence's

/// One row of steps for an track in a pattern
#[derive(Serialize, Deserialize, Clone)]
pub struct Sequence {
    pub track_id: u32,
    pub steps: Vec<Note>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Note {
    pub velocity: f32, // 0.0 = off, >0.0 = on
    pub pitch: u8,     // midi note 0-127, 60 = middle C5
}

impl Note {
    pub const DEFAULT: Self = Self {
        velocity: 0.0,
        pitch: 60,
    };
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

pub fn path_to_preview(track_path: &str, seconds: usize) -> Vec<f32> {
    let mut reader = match hound::WavReader::open(track_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to load {}: {}", track_path, e);
            return Vec::new();
        }
    };
    let spec = reader.spec();
    let divisor = 1 << (spec.bits_per_sample - 1);
    let max_samples = spec.sample_rate as usize * spec.channels as usize * seconds;
    reader
        .samples::<i32>()
        .filter_map(|s| s.ok())
        .take(max_samples)
        .map(|s| s as f32 / divisor as f32)
        .collect()
}

pub fn count_fs_rows(
    dir: &std::path::Path,
    expanded_dirs: &std::collections::HashSet<std::path::PathBuf>,
    fs_cache: &std::collections::HashMap<std::path::PathBuf, Vec<(std::path::PathBuf, bool)>>,
) -> usize {
    let Some(entries) = fs_cache.get(dir) else {
        return 0;
    };
    let mut count = entries.len();
    for (path, is_dir) in entries {
        if *is_dir && expanded_dirs.contains(path) {
            count += count_fs_rows(path, expanded_dirs, fs_cache);
        }
    }
    count
}
