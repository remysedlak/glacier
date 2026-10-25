//! Project details are stored in .toml files
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::thread::JoinHandle;

const SUPPORTED_AUDIO_EXTENSIONS: &[&str] = &["wav", "mp3", "flac", "aiff", "ogg"];
pub const MAX_VOICES_PER_TRACK: usize = 64;

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
    pub fn save_to_toml(&self, file_path: &str) -> Result<(), String> {
        let path = std::path::Path::new(file_path);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
            }
        }

        let text =
            toml::to_string(self).map_err(|e| format!("failed to serialize project: {e}"))?;
        #[cfg(windows)]
        {
            return std::fs::write(path, text)
                .map_err(|e| format!("failed to write {}: {e}", path.display()));
        }
        #[cfg(not(windows))]
        {
            let temporary_path = path.with_extension(format!(
                "{}.{}.tmp",
                path.extension()
                    .and_then(|extension| extension.to_str())
                    .unwrap_or("toml"),
                std::process::id()
            ));
            std::fs::write(&temporary_path, text)
                .map_err(|e| format!("failed to write {}: {e}", temporary_path.display()))?;
            if let Err(error) = std::fs::rename(&temporary_path, path) {
                let _ = std::fs::remove_file(&temporary_path);
                return Err(format!("failed to replace {}: {error}", path.display()));
            }
            Ok(())
        }
    }
    /// Use the default DAW file
    pub fn default_project_file() -> String {
        "assets/projects/new_project.toml".to_string()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SavePurpose {
    Regular,
    ProjectSwitch,
    Close,
}

pub struct ProjectLoadResult {
    pub project: Project,
    pub tracks: Vec<Track>,
    pub path: String,
}

/// Read project metadata and its referenced audio files away from the UI thread.
pub fn spawn_project_load(
    path: String,
    use_default_if_missing: bool,
) -> Receiver<Result<ProjectLoadResult, String>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let project = match get_project(&path) {
            Some(project) => project,
            None if use_default_if_missing => Project::default(),
            None => {
                let _ = sender.send(Err(format!("could not read project file {path}")));
                return;
            }
        };
        let tracks = get_tracks(&project);
        let _ = sender.send(Ok(ProjectLoadResult {
            project,
            tracks,
            path,
        }));
    });
    receiver
}

pub struct ProjectSaveJob {
    pub project: Project,
    pub path: String,
    pub purpose: SavePurpose,
    pub revision: u64,
}

pub struct ProjectSaveResult {
    pub path: String,
    pub purpose: SavePurpose,
    pub result: Result<(), String>,
    pub revision: u64,
}

enum ProjectSaveMessage {
    Save(ProjectSaveJob),
    Shutdown,
}

/// Long-lived worker for project serialization and file writes.
pub struct ProjectSaveWorker {
    sender: Sender<ProjectSaveMessage>,
    results: Receiver<ProjectSaveResult>,
    thread: Option<JoinHandle<()>>,
}

impl ProjectSaveWorker {
    pub fn new() -> Self {
        let (sender, jobs) = mpsc::channel();
        let (result_sender, results) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            while let Ok(message) = jobs.recv() {
                match message {
                    ProjectSaveMessage::Save(job) => {
                        let ProjectSaveJob {
                            project,
                            path,
                            purpose,
                            revision,
                        } = job;
                        let result = project.save_to_toml(&path);
                        if result_sender
                            .send(ProjectSaveResult {
                                path,
                                purpose,
                                result,
                                revision,
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                    ProjectSaveMessage::Shutdown => break,
                }
            }
        });

        Self {
            sender,
            results,
            thread: Some(thread),
        }
    }

    pub fn submit(&self, job: ProjectSaveJob) -> Result<(), String> {
        self.sender
            .send(ProjectSaveMessage::Save(job))
            .map_err(|_| "project save worker has stopped".to_string())
    }

    pub fn try_result(&self) -> Result<ProjectSaveResult, TryRecvError> {
        self.results.try_recv()
    }
}

impl Drop for ProjectSaveWorker {
    fn drop(&mut self) {
        let _ = self.sender.send(ProjectSaveMessage::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
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

pub fn next_id<T>(ids: impl Iterator<Item = u32>, wrap: impl Fn(u32) -> T) -> T {
    ids.max().map(|m| wrap(m + 1)).unwrap_or(wrap(0))
}

/// Different types of audio elements that can be placed on the playlist timeline
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
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
    pub track: u32,
    pub start_step: u32,            // what step does this patterns start at?
    pub length: u32,                // how long is this block? (cut/extended?)
    pub block_type: AudioBlockType, // pattern/instrument/mixing
    pub is_muted: bool,
}

impl AudioBlock {
    pub fn duplicate(&self, id: AudioBlockID, track: u32, step: u32) -> AudioBlock {
        let mut new_block = self.clone();
        new_block.id = id;
        new_block.track = track;
        new_block.start_step = step;
        // new_block.name = format!("{} Copy", self.name);
        new_block
    }
}

/// Runtime Track object
#[derive(Clone)]
pub struct Track {
    // persistent track information (serializable)
    pub data: TrackData,
    // recreated in memory every session
    pub samples: Arc<[f32]>,
    pub mono_samples: Arc<[f32]>,
    pub voice_pool: VoicePool,
    pub show_velocity: bool,
    pub rms_l: f32,
    pub rms_r: f32,
    pub peak_hold: f32,
}

impl Track {
    /// build track with data at default states
    pub fn from_data(data: TrackData, samples: Vec<f32>) -> Track {
        let mono_samples: Vec<f32> = if data.channels == 1 {
            Vec::new()
        } else {
            samples
                .chunks(2)
                .map(|pair| (pair[0] + pair[1]) / 2.0)
                .collect()
        };
        Track {
            samples: samples.into(),
            mono_samples: mono_samples.into(),
            data,
            voice_pool: VoicePool::default(),
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
pub fn spawn_track_load(path_str: String) -> Receiver<Track> {
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
        tx.send(Track::from_data(data, samples)).ok();
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
pub fn path_to_preview(track_path: &str, seconds: usize) -> (Vec<f32>, u16) {
    let mut reader = match hound::WavReader::open(track_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to load {}: {}", track_path, e);
            return (Vec::new(), 2);
        }
    };
    let spec = reader.spec();
    let divisor = 1 << (spec.bits_per_sample - 1);
    let max_samples = spec.sample_rate as usize * spec.channels as usize * seconds;
    let channels = spec.channels;
    let preview = reader
        .samples::<i32>()
        .filter_map(|s| s.ok())
        .take(max_samples)
        .map(|s| s as f32 / divisor as f32)
        .collect();

    (preview, channels)
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
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| SUPPORTED_AUDIO_EXTENSIONS.contains(&ext))
}

/// Fixed-capacity, Struct-of-Arrays voice pool for one track.
/// Column-major layout — intended to support future SIMD processing, not yet implemented.
#[derive(Clone)]
pub struct VoicePool {
    pub positions: [f32; MAX_VOICES_PER_TRACK],
    pub rates: [f32; MAX_VOICES_PER_TRACK],
    pub current_volumes: [f32; MAX_VOICES_PER_TRACK],
    pub target_volumes: [f32; MAX_VOICES_PER_TRACK],
    pub stop_at_frames: [Option<f32>; MAX_VOICES_PER_TRACK],
    pub active: [bool; MAX_VOICES_PER_TRACK],
    pub age: [u32; MAX_VOICES_PER_TRACK],
    active_scratch: [usize; MAX_VOICES_PER_TRACK],
    active_len: usize,
    voice_age_counter: u32,
}

impl Default for VoicePool {
    fn default() -> Self {
        VoicePool {
            positions: [0.0; MAX_VOICES_PER_TRACK],
            rates: [0.0; MAX_VOICES_PER_TRACK],
            current_volumes: [0.0; MAX_VOICES_PER_TRACK],
            target_volumes: [0.0; MAX_VOICES_PER_TRACK],
            stop_at_frames: [None; MAX_VOICES_PER_TRACK],
            active: [false; MAX_VOICES_PER_TRACK],
            age: [0; MAX_VOICES_PER_TRACK],
            active_scratch: [0; MAX_VOICES_PER_TRACK],
            active_len: 0,
            voice_age_counter: 0,
        }
    }
}

impl VoicePool {
    /// Refill the scratch buffer with indices of currently-active voices.
    /// Call once per callback/buffer before iterating — not once per sample.
    pub fn refresh_active_indices(&mut self) {
        self.active_len = 0;
        for i in 0..MAX_VOICES_PER_TRACK {
            if self.active[i] {
                self.active_scratch[self.active_len] = i;
                self.active_len += 1;
            }
        }
    }

    pub fn active_indices(&self) -> &[usize] {
        &self.active_scratch[..self.active_len]
    }

    /// Finds a free slot, or steals the oldest active one if the pool is full.
    pub fn spawn(
        &mut self,
        position: f32,
        rate: f32,
        current_volume: f32,
        target_volume: f32,
        stop_at_frame: Option<f32>,
    ) -> usize {
        let slot = (0..MAX_VOICES_PER_TRACK)
            .find(|&i| !self.active[i])
            .unwrap_or_else(|| {
                (0..MAX_VOICES_PER_TRACK)
                    .min_by_key(|&i| self.age[i])
                    .unwrap()
            });

        self.positions[slot] = position;
        self.rates[slot] = rate;
        self.current_volumes[slot] = current_volume;
        self.target_volumes[slot] = target_volume;
        self.stop_at_frames[slot] = stop_at_frame;
        self.active[slot] = true;
        self.voice_age_counter += 1;
        self.age[slot] = self.voice_age_counter;
        slot
    }
}
