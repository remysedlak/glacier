//! Project details are stored in .toml files
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Receiver;

/// Project store a song/session into memory and on file
#[derive(Serialize, Deserialize, Clone)]
pub struct Project {
    /// Name of the project
    pub name: String,
    /// Beats per minute
    pub bpm: f32,
    /// Amplitude of master channel (0-1.0)
    pub master_volume: f32,
    /// List of audio events
    pub audio_blocks: Vec<AudioBlock>,
    /// List of instruments
    pub tracks: Vec<TrackData>,
    /// List of patterns
    pub patterns: Vec<PatternData>,
}

impl Project {
    /// Create a new project
    pub fn new(
        name: String,
        bpm: f32,
        master_volume: f32,
        tracks: &[Track],
        patterns: Vec<PatternData>,
        audio_blocks: Vec<AudioBlock>,
    ) -> Project {
        Project {
            name,
            bpm,
            master_volume,
            tracks: tracks.iter().map(|track| track.data.clone()).collect(),
            patterns,
            audio_blocks,
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
    /// Return the default empty "New Project" for user's opening the app.
    fn default() -> Project {
        Project {
            name: "New Project".to_string(),
            bpm: 120.0,
            master_volume: 1.00,
            audio_blocks: vec![],
            tracks: vec![],
            patterns: vec![PatternData {
                id: PatternID(0),
                name: "Pattern 1".to_string(),
                sequences: vec![],
            }],
        }
    }
}

/// Different types of audio elements that can be placed on the playlist timeline
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", content = "id")]
pub enum AudioBlockType {
    Sample(TrackID),    // Instrument
    Pattern(PatternID), // Pattern
    Mixing,             // Automation
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// newtype for Pattern ID
pub struct PatternID(pub u32);
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// newtype for Track ID
pub struct TrackID(pub u32);
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// newtype for AudioBlock ID
pub struct AudioBlockID(pub u32);

/// AudioBlocks are how audio elements are timed within a playlist
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AudioBlock {
    pub id: AudioBlockID,
    pub track_id: TrackID,
    pub start_step: u32,            // what step does this patterns start at?
    pub length: u32,                // how long is this block? (cut/extended?)
    pub block_type: AudioBlockType, // pattern/instrument/mixing
    pub is_muted: bool,
}

/// Runtime Track object
#[derive(Clone)]
pub struct Track {
    // persistent track information
    pub data: TrackData,
    // recreated in memory every session
    pub samples: Vec<f32>,
    pub voices: Vec<Voice>,
    pub show_velocity: bool,
    pub rms_l: f32,
    pub rms_r: f32,
    pub peak_hold: f32,
}

impl Track {
    /// build track with data at default states
    pub fn from_data(data: TrackData, samples: Vec<f32>) -> Track {
        Track {
            samples,
            data,
            voices: vec![],
            show_velocity: false,

            rms_l: 0.0,
            rms_r: 0.0,
            peak_hold: 0.0,
        }
    }
    /// Toggle the mute flag of an instrument
    pub fn mute(&mut self) {
        self.data.is_muted = !self.data.is_muted;
    }
}

/// Track metadata stored on disk
#[derive(Serialize, Deserialize, Clone)]
pub struct TrackData {
    pub id: TrackID,
    pub name: String,
    pub path: String,
    pub is_muted: bool,
    pub channels: u16, // new — 1 = mono, 2 = stereo, from WAV header
    pub track_volume: f32,
    // default 60 - C5
    pub root_note: u8,
}

/// Patterns store a set of sequences
#[derive(Serialize, Deserialize, Clone)]
pub struct PatternData {
    pub id: PatternID,
    pub name: String,
    // vec  = { track_id , vec { notes } }, { track_id , vec { notes } }, { track_id , vec { notes } }
    pub sequences: Vec<Sequence>,
}

impl PatternData {
    pub fn duplicate(&self, id: PatternID) -> PatternData {
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
    pub track_id: TrackID,
    pub steps: Vec<Note>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
/// One midi note
pub struct Note {
    // 0.0 = off, >0.0 = on
    pub velocity: f32,
    // midi note 0-127, 60 = middle C5
    pub pitch: u8,
}

impl Note {
    // C5
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
        .map(|track| {
            let (samples, channels) = path_to_vector(&track.path);
            let mut data = track.clone();
            data.channels = channels; // keep TrackData in sync with actual file
            Track::from_data(data, samples)
        })
        .collect()
}

/// Helper method for starting a thread to handle loading a new track to the project from the file system
pub fn spawn_track_load(path_str: String) -> Receiver<(TrackData, Vec<f32>)> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let (samples, channels) = path_to_vector(&path_str);
        let name = std::path::Path::new(&path_str)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let data = TrackData {
            id: TrackID(0),
            path: path_str,
            name,
            channels,
            track_volume: 1.0,
            is_muted: false,
            root_note: 60,
        };
        tx.send((data, samples)).ok();
    });
    rx
}

/// load a track's float data from it's file path
pub fn path_to_vector(track_path: &str) -> (Vec<f32>, u16) {
    let mut reader = match hound::WavReader::open(track_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to load {}: {}", track_path, e);
            return (Vec::new(), 2);
        }
    };
    let spec = reader.spec();
    let divisor = 1 << (spec.bits_per_sample - 1);
    let samples = reader
        .samples::<i32>()
        .filter_map(|s| s.ok())
        .map(|s| s as f32 / divisor as f32)
        .collect();
    (samples, spec.channels)
}

/// Return preview clip of a track's audio data from file path
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

/// return how many file system rows are in a Path
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

/// Returns true if a file path is a playable music file
pub fn is_audio_file(path: &std::path::Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("wav" | "mp3" | "flac" | "aiff" | "ogg") // how can i create a constant to store supported audio files?
    )
}

#[derive(Clone)]
/// One track note instant
///
/// spawned and killed
/// TODO: figure out how voices should be managed in memory
pub struct Voice {
    pub position: f32,
    pub is_playing: bool,
    pub playback_rate: f32,
    pub current_volume: f32,
    pub target_volume: f32,
    pub stop_at_frame: Option<f32>,
}
