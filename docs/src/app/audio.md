# The audio thread

The audio threads main job is to satisfy commands from the UI and maintain a low latency audio stream.

## Voice Model — Polyphony

### Voice vs Track
```rust
pub struct Voice {
    pub position: f32,
    pub is_playing: bool,
    pub playback_rate: f32,
    pub current_volume: f32,
    pub target_volume: f32,
    pub stop_at_frame: Option<f32>,  // wired in and enforced — see below
}

pub struct Track {
    pub data: TrackData,
    pub samples: Vec<f32>,
    pub voices: Vec<Voice>,   // replaces the single position/is_playing/etc. fields
    pub show_velocity: bool,
    pub rms_l: f32,
    pub rms_r: f32,
    pub peak_hold: f32,
}
```
In glacier, a `Track` is a unique loaded instrument as samples. A `Voice` represents one instance of a track playing in the `Project`.
Voices were created so that a Track can have overlapping sounds and different playback lengths.

### Triggering — always push, no voice stealing yet
Both trigger sites (`AudioBlockType::Sample` trigger loop, pattern-note trigger loop in
audio.rs) `track.voices.push(Voice { ... })` on every trigger — no search for a free/
reusable voice, no cap on simultaneous voices. This is a deliberate simplification:
correct, but voices from finished notes only get removed via the cleanup below, and there's
no limit on how many can pile up if triggers fire faster than they finish. Real "voice
stealing" (reusing the oldest/quietest voice once at a cap) is a known future improvement,
not yet needed.

### Cleanup — retain in the mixing loop, not a separate pass
```rust
for track in &mut tracks {
    if !track.data.is_muted {
        for voice in &mut track.voices {
            if voice.is_playing { /* mix, same math as before, per-voice */ }
        }
        track.voices.retain(|v| v.is_playing);
    }
}
```
`retain` runs once per track per audio callback — O(n) in voice count (small), no
reallocation in steady state (only shrinks in-place). This is NOT garbage collection;
it's a single filter pass, cheap enough for the real-time audio callback.

### stop_at_frame — now wired in, both units matter
At trigger time (`AudioBlockType::Sample` arm), `stop_at_frame` is computed as
`audio_block.length as f32 * samples_per_step * 2.0`. The `* 2.0` matters:
`glacier_dsp::samples_per_step` returns plain frames-per-step, but `voice.position`
and `track.samples.len()` both operate in raw interleaved-array-index space (confirmed
by `voice.position += 2.0 * voice.playback_rate` and `pos + 3 >= track.samples.len()`
already stepping/indexing by 2 per stereo frame) — so frames had to be doubled to
match. In the mixing loop, the cutoff check became:
```rust
let stop = voice.stop_at_frame.unwrap_or(track.samples.len() as f32);
if pos as f32 + 3.0 >= stop.min(track.samples.len() as f32) {
    voice.is_playing = false;
}
```
The `.min()` guards a block resized longer than the actual sample from reading past
`track.samples`'s real length. Pattern-triggered notes still pass `stop_at_frame: None`
(full sample plays) — not yet revisited, see Known Open Issues.

### rms_l/rms_r/peak_hold stay on Track, not Voice
These are a summed/aggregate signal across all of a track's currently-playing voices,
not per-voice state — they stay put, unlike position/is_playing/etc.

---

## Command Loop — how the UI talks to audio

### The shape
Every audio callback starts by draining the incoming command queue before touching
any samples:
```rust
while let Some(cmd) = consumer.try_pop() {
    match cmd {
        AudioCommand::ToggleStep(...) => { /* mutate this thread's copy */ }
        // ...
    }
}
```
`consumer`/`producer` are the two `ringbuf` halves — the UI thread only ever sends
`AudioCommand`s in, and this thread only ever sends `UiCommand`s back out. Nothing
is shared behind a `Mutex`; the ring buffer is the entire synchronization mechanism.
This runs at the top of *every* callback, so command latency is bounded by audio
buffer size, not by anything the UI does.

### Audio thread owns the real data
`tracks`, `patterns`, `audio_blocks`, `bpm`, `master_volume` — all of it lives as
plain local variables captured by the callback closure, not behind any handle the
UI can reach. The UI's `Graphics` struct holds a *mirror* of the same shapes, updated
only by `UiCommand`s sent back from here. This thread is the sole source of truth;
see the Data Model / ID System chapter for why that matters for id-minting and
delete-cascades specifically.

### The confirmation pattern
Almost every mutating arm follows the same two-step shape: mutate the local copy,
then `producer.try_push(UiCommand::SomethingUpdated(...))`. The UI never assumes a
click succeeded — it waits for the matching `UiCommand` to actually update what it
draws. `try_push` can fail silently (queue full) and that's treated as acceptable:
the alternative (blocking the audio callback until the UI catches up) would risk an
audible glitch, which is worse than an occasionally-dropped UI update.

### Why lookups are all `.find()`, never `tracks[track_id]`
Every id here (pattern_id, track_id, audio_block_id) is a stable identity, not a position in its Vec. .iter_mut().find(|x| x.id == id) (or .iter().position(...) when the id needs to be removed by position, as in DeleteTrack) is the pattern everywhere — no arm bracket-indexes by an id-shaped value directly.

---

## Playback Timing — steps, not wall-clock

### samples_per_step is the single unit of time
The whole sequencer's notion of "time" reduces to one function:
glacier_dsp::samples_per_step(sample_rate, bpm) — how many raw audio frames make up one step (a step = a 1/16 note; four steps per beat, per the project's convention)

### sample_counter drives step-advance, once per callback
```rust
sample_counter += data.len() as f32 / 2.0;   // frames requested this callback
if sample_counter >= samples_per_step {
    sample_counter = 0.0;
    current_step = (current_step + 1) % total_steps;
    // ...fire triggers for the new step
}
```
`total_steps` is recomputed from `audio_blocks` each time (`max(start_step + length)`),
not stored — so it always reflects the current playlist even if blocks were added/
removed mid-playback.

### Two trigger paths, one per AudioBlockType
- **`AudioBlockType::Sample`** — fires exactly once, the callback the step matches
  `audio_block.start_step`, and pushes one `Voice` with a computed `stop_at_frame`
  (see Voice Model).
- **`AudioBlockType::Pattern`** — resolved through `triggers`, a `Vec` built each
  step by mapping every active pattern-block to its pattern's notes at the current
  `local_step`, filtered to non-zero velocity. Each hit becomes a `Voice` with
  `stop_at_frame: None` (plays out the full sample) and a `playback_rate` derived
  from `glacier_dsp::semitones_to_rate(pitch, root_note)`.

Both paths converge on the same `Voice` push — the mixing loop doesn't know or care
which path a voice came from.

---

## Metering & Spectrum — reporting without stalling audio

### RMS/peak — smoothed, not raw
Per-sample, both master and per-track RMS use `glacier_dsp::smooth_toward(current,
target, coeff)` — an exponential decay toward the instantaneous squared sample value,
not the raw value itself. This is why every `UiCommand::*Level` push takes `.sqrt()`
of the accumulated value — RMS is `sqrt(mean(x²))`, and the running-average happens
in squared space before the one `.sqrt()` at report time.

### Reporting is throttled, mixing isn't
```rust
meter_counter += data.len() / 2;
if meter_counter >= 1024 {
    meter_counter = 0;
    producer.try_push(UiCommand::MasterLevel(...)).ok();
    // ... per-track levels ...
}
```
Smoothing runs every sample (needed for accuracy), but pushing `UiCommand`s to the UI
only happens roughly every 1024 frames — sending a `UiCommand` every single sample
would flood the ring buffer for no visual benefit, since the UI can't render faster
than a frame anyway.

### Spectrum — buffered across callbacks, computed once full
`spectrum_buffer` accumulates the mixed mono signal (`(left + right) * 0.5`) across
however many callbacks it takes to reach `SPECTRUM_WINDOW` (2048) samples — a single
callback's `data` buffer is usually much smaller than that. Once full: apply a Hann
window (`glacier_dsp::hann_window`), run the FFT (`SpectrumAnalyzer::process`),
convert magnitudes to dB (correcting for the window's own attenuation via
`window_compensation`), push the result, clear the buffer, repeat. This means
spectrum updates arrive less often than level meters, proportional to `SPECTRUM_WINDOW`
vs. `1024` — an intentional tradeoff, not a bug, since FFT resolution needs the
larger window.

---

## Shutdown — fading, not cutting

### Why a fade at all
Stopping playback by simply dropping the stream produces an audible click — cutting
a waveform mid-cycle is a discontinuity the speaker reproduces as a pop. Instead,
`Shutdown`/`ShutdownWithoutSaving` set `is_shutting_down = true` and let the *next*
several callbacks ramp `shutdown_volume` down from 1.0, multiplying it into every
sample's gain alongside the normal volume factors.

### Decrement lives inside the per-sample loop, not per-callback
```rust
for sample in data.chunks_mut(2) {
    if is_shutting_down {
        shutdown_volume -= 0.0001;
        if shutdown_volume <= 0.0 {
            producer.try_push(UiCommand::ShutdownComplete).ok();
        }
    }
    // ...
}
```
Decrementing once per callback instead of once per sample would make fade length
depend on buffer size (different across devices/drivers) rather than a fixed sample
count — putting it in the per-sample loop keeps fade duration consistent regardless
of how big `data` happens to be on a given callback.

### Save-before-confirm, not confirm-before-save
Both shutdown variants that save (`Shutdown`) call `project.save_to_toml` and push
`UiCommand::SaveComplete` *before* checking `is_playing` to decide whether to also
send `ShutdownComplete` immediately. If playback is still active, `ShutdownComplete`
instead waits for the fade-out loop above to reach zero — the UI shouldn't tear down
the audio stream out from under an in-progress fade.
