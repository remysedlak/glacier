# The audio thread

The audio thread's main job is to satisfy commands from the UI and maintain a low latency audio stream.

## Voice Model — Polyphony

### VoicePool, not Vec<Voice>

```rust
pub const MAX_VOICES_PER_TRACK: usize = 64;

pub struct VoicePool {
    pub positions: [f32; MAX_VOICES_PER_TRACK],
    pub rates: [f32; MAX_VOICES_PER_TRACK],
    pub current_volumes: [f32; MAX_VOICES_PER_TRACK],
    pub target_volumes: [f32; MAX_VOICES_PER_TRACK],
    pub stop_at_frames: [Option<f32>; MAX_VOICES_PER_TRACK],
    pub active: [bool; MAX_VOICES_PER_TRACK],
    pub age: [u32; MAX_VOICES_PER_TRACK],
    active_scratch: Vec<usize>,
    voice_age_counter: u32,
}

pub struct Track {
    pub data: TrackData,
    pub samples: Vec<f32>,
    pub mono_samples: Vec<f32>,
    pub voice_pool: VoicePool,
    pub show_velocity: bool,
    pub rms_l: f32,
    pub rms_r: f32,
    pub peak_hold: f32,
}
```

This used to be `Vec<Voice>`, where `Voice` was one bundled struct with all six fields together. Got rewritten into a fixed-size, struct-of-arrays pool — every voice's `position` lives in one flat array, every `rate` in another, etc. The old `Voice` type doesn't exist anymore; it got fully replaced, not just wrapped.

Two reasons for the rewrite: capping polyphony at a real number instead of letting `Vec<Voice>` grow forever, and laying the data out so it's actually possible to batch it with SIMD later (not done yet, just the groundwork).

### Spawning — find a free slot, or steal the oldest

```rust
pub fn spawn(&mut self, position: f32, rate: f32, current_volume: f32, target_volume: f32, stop_at_frame: Option<f32>) -> usize {
    let slot = (0..MAX_VOICES_PER_TRACK)
        .find(|&i| !self.active[i])
        .unwrap_or_else(|| {
            (0..MAX_VOICES_PER_TRACK).min_by_key(|&i| self.age[i]).unwrap()
        });
    // write all five fields into that slot, bump age, mark active
    slot
}
```

No more unbounded pushing. If there's a free slot, use it. If the pool's full (64 voices already going on one track), kill the oldest one and reuse its slot. `age` is just a counter that only ever goes up — every spawn stamps the slot with the current counter value, so "oldest" means "lowest age," not anything to do with pitch or position.

### Cleanup — no more retain()

Old version swept dead voices out of a `Vec` every callback with `.retain()`. New version doesn't need to — a dead voice just gets `active[i] = false` the instant it finishes, and that's it. No sweep pass. `refresh_active_indices()` runs once per track per callback, refilling a scratch buffer with whichever indices are currently active, and the mixing loop walks that scratch buffer instead of the raw 64 slots.

```rust
track.voice_pool.refresh_active_indices();
for i in 0..track.voice_pool.active_indices().len() {
    let voice = track.voice_pool.active_indices()[i];
    mix_voice(track, voice, shutdown_volume, master_volume, sample);
}
```

### stop_at_frame and the mono/stereo mess

This one took a while to get right. `frame_advance` and `frame_lookahead` both exist specifically because stereo data is interleaved (`L0 R0 L1 R1...`, 2 slots per frame) and mono data isn't (`S0 S1 S2...`, 1 slot per frame). Every number that used to be a hardcoded `2.0` or `3.0` — the position-advance step, the bounds-check lookahead, the frame alignment — had to become channel-aware or mono tracks would read garbage.

```rust
pub fn frame_advance(channels: u16) -> f32 {
    if channels == 1 { 1.0 } else { 2.0 }
}

pub fn frame_lookahead(channels: u16) -> usize {
    if channels == 1 { 1 } else { 3 }
}
```

`mix_voice` uses both of these now instead of bare constants:

```rust
let channels = track.data.channels;
let pos = if channels == 1 {
    track.voice_pool.positions[voice] as usize
} else {
    align_to_frame(track.voice_pool.positions[voice])
};
let lookahead = frame_lookahead(channels);
let stop = track.voice_pool.stop_at_frames[voice].unwrap_or(track.samples.len() as f32);

if pos + lookahead >= stop.min(track.samples.len() as f32) as usize {
    track.voice_pool.active[voice] = false;
} else {
    // smooth volume, interpolate, mix into sample, advance position
}
```

`stop_at` itself (computed at trigger time for sample blocks) also needs `frame_advance` baked in, same reason — it has to be in the same index-space as `position` for the comparison above to make sense:

```rust
let stop_at = audio_block.length as f32 * frame_advance(track.data.channels) * step_duration;
```

Pattern-triggered notes still pass `stop_at_frame: None` — they just play out until they run off the end of the actual sample data.

### Interpolation — one function, branches internally

`interpolate(samples, position, channels)` replaced two separate functions (`interpolate_mono`/`interpolate_stereo`) that used to get called with a manual `if channels == 1 {...} else {...}` at every call site. Now it's one call, branches inside:

```rust
pub fn interpolate(samples: &[f32], position: f32, channels: u16) -> (f32, f32) {
    if channels == 1 {
        interpolate_mono(samples, position)
    } else {
        interpolate_stereo(samples, position)
    }
}
```

Both `interpolate_mono` and `interpolate_stereo` still exist underneath it, they're just not called directly from `audio.rs` anymore.

### rms_l/rms_r/peak_hold stay on Track, not in the pool

Same as before — these are aggregate signal across all of a track's currently-playing voices, summed together, not per-voice state. They don't belong in `VoicePool` and never did.

---

## Command Loop — how the UI talks to audio

Still the same shape as it's always been. Every callback drains the command queue before touching any samples:

```rust
while let Some(cmd) = consumer.try_pop() {
    handle_command(cmd, &mut engine_state);
}
```

`consumer`/`producer` are the two `ringbuf` halves. UI sends `AudioCommand`s in, audio thread sends `UiCommand`s back out, nothing's behind a `Mutex`. The audio thread owns the real `EngineState` — tracks, patterns, audio_blocks, bpm, all of it — and the UI's `Graphics` struct is just a mirror, updated only by whatever `UiCommand`s come back.

Preview playback now carries its own channel count through the whole chain, since preview files aren't guaranteed to be the same channel layout as whatever track triggered them:

```rust
AudioCommand::PreviewSample(samples, channels) => {
    engine_state.preview_samples = samples;
    engine_state.preview_channels = channels;
    engine_state.preview_position = 0.0;
}
```

Lookups are still always `.find()` by id, never bracket-indexed — nothing about that changed.

---

## Playback Timing — steps, not wall-clock

`samples_per_step(sample_rate, bpm)` is still the one unit of time the whole sequencer runs on. `sample_counter` still drives step-advance once per callback, except the divisor isn't a hardcoded `2.0` anymore — it uses whatever the actual output device negotiated:

```rust
sample_counter += data.len() as f32 / output_channels as f32;
```

Same deal for the meter-update frame count a bit further down. Both of these used to assume exactly 2 output channels; now they use `output_channels`, which comes from the real negotiated `stream_config.channels`. Anything that isn't stereo (mono gets rejected outright at startup, anything 3+ channels works fine with the extra channels left silent) used to silently desync the playhead from real audio time. Fixed now.

`current_step` and `total_steps` both got a `.max(1)` guard added before any subtraction happens, since an empty or all-zero-length playlist used to be able to underflow `0usize - 1` and panic.

---

## Metering & Spectrum — reporting without stalling audio

Mostly unchanged, but it's not all crammed into one closure anymore. `push_meter_updates` and `push_spectrum_frame` are real, named functions now, living in `audio/metering.rs`, called from `fill_f32` instead of being inlined 20-line blocks in the middle of the callback.

RMS still smooths in squared space with `smooth_toward`, still takes one `.sqrt()` at report time. Reporting still throttles to roughly every 1024 frames, same reasoning as before — no point flooding the ring buffer faster than the UI can render a frame.

`master_peak` gets reset to `0.0` inside `push_meter_updates`, right after the `MasterLevel` push. Worth saying explicitly since it's easy to assume a running max like that just accumulates forever if you're skimming — it doesn't, it's a real reset every window.

Spectrum still buffers the mono mixdown across however many callbacks it takes to hit `SPECTRUM_WINDOW` (2048 samples), still Hann windows it, runs the FFT, corrects for window attenuation, converts to dB. `SPECTRUM_WINDOW` moved from a local `const` inside `init()` to a module-level `const` in `audio.rs`, since `push_spectrum_frame` needed to see it too now that it's a separate function.

---

## Shutdown — fading, not cutting

Same idea as before — drop the stream outright and you get a click, so instead `is_shutting_down` ramps `shutdown_volume` down from 1.0 over the next several callbacks.

Two real bugs got fixed here. First, the fade used to be a flat `-0.0001` per sample, which meant fade time depended on sample rate — ~226ms on a 44.1kHz device, more like 104ms on 96kHz. Now it's computed from the actual sample rate so fade time is a fixed, device-independent duration:

```rust
let fade_decrement = 1.0 / (FADE_TIME_SECS * engine_state.sample_rate);
```

Second, `shutdown_volume` never used to get reset after the fade finished — it'd just sit at whatever small negative number it landed on. Harmless as long as shutdown is genuinely terminal, but if anything ever re-triggers shutdown without rebuilding the stream, every sample afterward gets multiplied by a negative number. Fixed:

```rust
if shutdown_volume <= 0.0 {
    producer.try_push(UiCommand::ShutdownComplete).ok();
    is_shutting_down = false;
    shutdown_volume = 1.0;
}
```

Preview playback now fades along with everything else during shutdown too — it used to only scale by `master_volume`, so a preview playing right as the app closed would cut off hard while the rest of the mix faded smoothly. Now `process_preview` takes `shutdown_volume` as well and multiplies it in, same as `mix_voice` already did.

Save-before-confirm is unchanged — `Shutdown` still saves and pushes `SaveComplete` before deciding whether to send `ShutdownComplete` immediately or wait for the fade to actually finish.
