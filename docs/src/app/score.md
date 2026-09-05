# Project & Data Model

A Glacier project is one `.toml` file on disk plus the runtime state built
from it. This chapter covers what that file actually contains, why the
in-memory model splits into two tiers, and how the two stay in sync across
save/load.

## Two Tiers: Persisted vs. Runtime

Every core concept in the song has a **persisted** shape (serializable,
written to the `.toml` file) and, for tracks specifically, a separate
**runtime** shape that wraps it with state that only exists while the app
is running.

```rust
// Persisted — serde-derived, written to disk as-is
struct TrackData {
    id: u32,
    name: String,
    path: String,
    is_muted: bool,
    channels: u16,
    track_volume: f32,
    root_note: u8,
}

// Runtime — wraps TrackData, adds state with no meaning at rest
struct Track {
    data: TrackData,
    samples: Vec<f32>,      // decoded WAV data, not stored in the toml
    voices: Vec<Voice>,     // currently-sounding notes, meaningless when saved
    show_velocity: bool,    // UI-only toggle
    rms_l: f32,
    rms_r: f32,
    peak_hold: f32,
}
```

`Track` is never serialized directly — `Project` only ever stores
`Vec<TrackData>`. This split exists because `samples` (a decoded WAV, often
large) and `voices`/meter state (transient, per-callback) have no business
in a save file; only `TrackData` describes what the track *is*, independent
of whatever's currently playing.

`PatternData` and `AudioBlock`, by contrast, don't need this split — they
have no meaningful "currently playing" runtime-only state, so they're used
directly, both as the live data the audio thread mutates and as what gets
written to disk.

## Project — the file itself

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

This struct's shape *is* the `.toml` schema — `#[derive(Serialize,
Deserialize)]` means whatever fields live here are exactly what gets
written and read. Notice `tracks: Vec<TrackData>`, not `Vec<Track>` — this
is the two-tier split above showing up directly in the save format.

`Project::default()` is what a brand-new project looks like: no tracks, no
blocks, and — deliberately — one empty pattern already present (`id: 0,
name: "Pattern 1"`), so the sequencer/piano roll always has something to
show rather than an empty list on first launch.

## AudioBlock & AudioBlockType — the playlist timeline

```rust
enum AudioBlockType {
    Sample(usize),   // plays a track's raw sample directly
    Pattern(usize),  // plays a pattern's sequenced notes
    Mixing,          // reserved for automation — not yet driving playback
}

struct AudioBlock {
    id: usize,
    track_id: usize,
    start_step: u32,
    length: u32,
    block_type: AudioBlockType,
}
```

An `AudioBlock` is a placement on the playlist timeline: *this* content,
starting at *this* step, lasting *this* many steps. `block_type` decides
whether "this content" means "trigger one sample" or "play a pattern's
notes" — same placement struct, different playback source.

`#[serde(tag = "kind", content = "id")]` on `AudioBlockType` is what makes
this readable in the saved `.toml` rather than an opaque tagged union — it
serializes as e.g. `kind = "Sample"`, `id = 3` instead of a positional
tuple.

> **Note on `track_id`'s type:** `AudioBlock.track_id` is `usize`, while
> `TrackData.id` is `u32` — matching them requires an explicit `as u32`/
> `as usize` cast at every lookup site (visible in `audio.rs`, e.g. `t.data.id
> == *sample_track_id as u32`). This isn't a bug, just an inconsistency
> worth knowing about if you're tracing a lookup that isn't matching —
> check the cast before assuming the id itself is wrong.

## Patterns, Sequences, Notes — the step grid

```rust
struct PatternData {
    id: usize,
    name: String,
    sequences: Vec<Sequence>,
}
struct Sequence {
    track_id: u32,      // which track this row belongs to
    steps: Vec<Note>,
}
struct Note {
    velocity: f32,      // 0.0 = off, >0.0 = on
    pitch: u8,          // MIDI note, 60 = middle C
}
```

A pattern is a set of `Sequence`s, one per track that has *any* notes in
that pattern — tracks with nothing programmed simply have no `Sequence`
entry at all, rather than a `Sequence` full of empty `Note`s. This is a
sparse representation, not a fixed grid: `Sequence.steps` only grows as far
as the last active note.

This sparsity is actively maintained, not just a save-time optimization —
every note-toggle in `audio.rs` ends with a trim:
```rust
while seq.steps.last().map(|n| n.velocity == 0.0).unwrap_or(false) {
    seq.steps.pop();
}
```
Toggling off the last active step in a row shrinks `steps` back down
immediately, rather than leaving trailing empty `Note`s around. `Note::DEFAULT`
(`velocity: 0.0, pitch: 60`) is the canonical "empty" value used both to pad
a row up to a new step index and to check against when trimming.

## Voice — not part of the saved model at all

```rust
struct Voice {
    position: f32,
    is_playing: bool,
    playback_rate: f32,
    current_volume: f32,
    target_volume: f32,
    stop_at_frame: Option<f32>,
}
```

Worth calling out explicitly: `Voice` never appears in `Project` and has no
serde derive. It's pure runtime — one per currently-sounding note, created
on trigger and dropped once finished (see the audio thread chapter's Voice
Model section for the full mechanics). A saved project has zero opinion
about what's currently playing.

## Save / Load Flow

**Loading:** `get_project(path)` reads and deserializes the `.toml` into a
`Project`. `get_tracks(project)` then converts each `TrackData` into a full
runtime `Track` by decoding its WAV file (`path_to_vector`) and wrapping
with empty runtime state — this is the one place `TrackData` becomes
`Track`.

**Saving:** `Project::new(...)` does the reverse — given the current
runtime `&[Track]`, it strips each down to just its `.data` field
(discarding samples/voices/meters) to rebuild a `Vec<TrackData>`, bundles it
with the current `patterns`/`audio_blocks`/`bpm`/`master_volume`, and
`save_to_toml` writes the result. This round-trip (`Track` → `TrackData` →
toml → `TrackData` → `Track`) is why nothing runtime-only can ever leak into
a save file — the conversion function simply doesn't have access to it.

Both directions go through the audio thread exclusively (see Command Loop
in the audio thread chapter) — `SaveProject`/`Shutdown` handlers build and
write the `Project` from the audio thread's own copy of tracks/patterns/
blocks, since that copy is the sole source of truth.
