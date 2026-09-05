# Synthesis & Playback Math

This chapter covers the small set of functions that drive timing and pitch
during playback — not signal generation in the oscillator sense, but the
math that decides *when* and *how fast* existing samples play back.

## Timing — Steps and BPM

```rust
pub fn samples_per_step(sample_rate: f32, bpm: f32) -> f32
```

Converts a tempo into a concrete sample count: how many audio frames make
up one sequencer step. Derivation, since the steps-per-beat convention
isn't obvious from the formula alone:

- 60 seconds per minute → `beats_per_second = bpm / 60.0`
- 4 steps per beat (project convention: one step = 1/16 note in 4/4 time)
  → `steps_per_second = beats_per_second * 4.0`
- `samples_per_second / steps_per_second` gives samples per step

This single function is the entire notion of "time" for the sequencer — the
audio thread's step-advance logic, playhead position, and sample cutoff
calculations all reduce to this number multiplied or divided against a
counter (see the audio thread chapter's Playback Timing section).

## Exponential Smoothing

```rust
pub fn smooth_toward(current: f32, target: f32, coeff: f32) -> f32
```

`current + (target - current) * coeff` — moves `current` a fraction of the
way toward `target` each call. This is the one primitive underlying several
otherwise-unrelated pieces of runtime state: voice volume ramping (avoiding
clicks when a note starts/stops), RMS meter smoothing, and the envelope
follower's per-sample decay (see Analysis chapter). A single small function
reused everywhere rather than each call site hand-rolling its own decay math.

Higher `coeff` reaches the target faster (more responsive, less smooth);
lower `coeff` is slower but smoother. There's no single "correct" value —
each call site picks a coefficient suited to how fast that particular thing
should respond (e.g. voice volume ramps use `0.01` for a gentle fade-in;
faster response would reintroduce the click this exists to avoid).

## Pitch → Playback Rate

```rust
pub fn semitones_to_rate(pitch: u8, root_note: u8) -> f32
```

`2^((pitch - root_note) / 12)` — converts a semitone interval into a
playback-speed multiplier. This is how one recorded sample plays back at
multiple pitches: a track's `root_note` is whatever pitch the sample was
recorded at (default 60, middle C), and triggering a different `pitch`
speeds up or slows down playback by the ratio that produces that interval.
12 semitones (one octave) doubles the rate; -12 halves it. This is the same
math as `voice.playback_rate` in the audio thread's `Voice` struct — this
function is where that value comes from at trigger time.

> **Tradeoff worth knowing:** this is simple rate-based pitch shifting, not
> a formant-preserving pitch shift — large intervals will noticeably change
> the sample's timbre and duration (higher pitch = shorter playback, lower
> pitch = longer), not just its pitch. Fine for drum/instrument samples
> within a reasonable range; would sound wrong for something like pitch-
> shifting a vocal by an octave.
