# Project & Data Model

A Glacier project is persisted as one TOML file. The app owns the editable
project values and creates save snapshots from them. Audio keeps a separate
runtime representation containing only what it needs to render playback.

## Persisted project model

The serializable `Project` contains the project name, tempo, master volume,
playlist blocks, track metadata, and patterns:

```rust
struct Project {
    name: String,
    bpm: f32,
    master_volume: f32,
    audio_blocks: Vec<AudioBlock>,
    tracks: Vec<TrackData>,
    patterns: Vec<PatternData>,
}
```

`TrackData` is the persisted description of a track: its stable ID, name,
source path, mute state, channel count, volume, and root note. Decoded audio,
waveform data, voices, and meters are runtime data and are not serialized.

## App and playback representations

The app's runtime `Track` combines `TrackData` with immutable shared sample
buffers (`Arc<[f32]>`), a prepared mono waveform, a fixed-capacity `VoicePool`,
UI state, and meter values. The audio engine receives an `AudioTrack`
projection with only playback fields: ID, channel count, mute/volume, root
note, shared samples, voice pool, and meters. Pattern names and track paths
stay in the app model; audio does not need them to play.

`PatternData` contains an ID, display name, and sequences. The audio-side
`AudioPattern` projection keeps the ID and sequences but omits the name.
`AudioBlock` describes a placement on the playlist: its ID, lane, start step,
length, mute state, and whether it references a sample track, pattern, or
mixing/automation content.

```rust
enum AudioBlockType {
    Sample(TrackID),
    Pattern(PatternID),
    Mixing,
}

struct AudioBlock {
    id: AudioBlockID,
    track: u32, // playlist lane
    start_step: u32,
    length: u32,
    block_type: AudioBlockType,
    is_muted: bool,
}
```

The `TrackID`, `PatternID`, and `AudioBlockID` newtypes make stable identities
distinct at the type level. `AudioBlock.track` is a lane/index value and is
not the block's identity.

Patterns use sparse sequences. A `Sequence` identifies a track and stores its
notes; rows with no notes need not exist, and trailing empty steps are trimmed
when a note is removed. `Note::DEFAULT` is the canonical empty step.

## Loading and saving

When a project is opened, a worker reads and deserializes its TOML and loads
referenced track audio. The app receives the `Project` and constructed
`Track`s, then initializes the playback engine and applies the loaded project
to its UI state. Loading another project replaces the active stream/engine
after the worker has returned successfully; the audio callback does not parse
files or decode samples.

For saves, the app builds a `Project` snapshot from its current editable state.
One long-lived save worker serializes the snapshot and writes the TOML file.
Results include the project revision captured by the snapshot, so completion
of an older save does not mark newer edits as saved. Track runtime data is
excluded because the snapshot contains `TrackData`, not the app's full runtime
`Track` values.

Structural edits follow the same ownership boundary: the app edits its project
state, then sends audio a playback-only update for the changed collection.
Audio swaps that collection at a callback boundary and returns the old data to
the app thread for destruction. Audio reports runtime playback facts such as
meters and playhead position, but does not send project mutations back as the
canonical state.

See [Audio Engine and Callback](./audio.md) for callback and queue behavior,
and [IDs and Project Authority](./ids.md) for ID allocation and edit rules.
