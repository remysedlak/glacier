# Synthesis & Playback Math

Small set of functions that drive timing, pitch, and reading samples during playback. Not oscillators or sound generation, just the math deciding when, how fast, and from where existing samples play.

## Timing — Steps and BPM

```rust
pub fn samples_per_step(sample_rate: f32, bpm: f32) -> f32
```

Turns a tempo into a sample count, how many audio frames make up one step. Derivation since the steps-per-beat thing isn't obvious just from the formula:

- 60 sec/min. `beats_per_second = bpm / 60.0`
- 4 steps per beat (1 step = 1/16 note in 4/4). `steps_per_second = beats_per_second * 4.0`
- `samples_per_second / steps_per_second` is samples per step

This is basically the whole concept of "time" for the sequencer. Step advance, playhead position, sample cutoffs, all of it comes back to this number.

## Exponential Smoothing

```rust
pub fn smooth_toward(current: f32, target: f32, coeff: f32) -> f32
```

`current + (target - current) * coeff`. Moves current toward target by some fraction each call. Used in three places that don't seem related at first — voice volume ramping so notes don't click on/off, RMS meter smoothing, envelope follower decay. Didn't want to write the same decay math three times so it's one function.

Higher coeff reaches target faster but less smooth, lower is smoother but slower. No universal right answer, depends what you're smoothing. Voice volume uses 0.01, a gentle fade. Go faster and the click comes back, which defeats the whole point.

Side note: decaying toward zero with this can land in denormal float territory, tiny non-zero values the CPU is slow at. That's what `flush_denormal` is for.

```rust
pub fn flush_denormal(x: f32) -> f32
```

`if x.abs() < 1e-15 { 0.0 } else { x }`. Once a value's quiet enough nobody could ever hear it, snap it to real zero. `smooth_toward` never actually reaches zero since it's exponential decay, so without this a value can sit decaying in denormal range basically forever, and every sample touching it runs 10-100x slower than normal. Wrapped around every `smooth_toward` call in the audio thread that's decaying to 0.

## Pitch → Playback Rate

```rust
pub fn semitones_to_rate(pitch: u8, root_note: u8) -> f32
```

`2^((pitch - root_note) / 12)`. Turns a semitone interval into a speed multiplier. One recorded sample, played at different pitches. `root_note` is whatever pitch the sample was actually recorded at (default 60, middle C), and triggering some other `pitch` speeds up or slows down playback by whatever ratio gets you there. Octave up doubles the rate, octave down halves it. This is where `voice_pool.rates[slot]` actually comes from at trigger time.

Worth knowing this isn't formant preserving, it's just speed changes. Big intervals change the sample's timbre and length too, not just pitch. Fine for drums and one shots, would sound bad pitch shifting a vocal an octave.

## Reading Between Samples — Interpolation

Position is a float, not an int. Any time playback_rate isn't exactly 1.0 (basically always since that's the whole point of pitch shifting) position lands between two real samples instead of on one. Interpolation guesses the value that would be there.

```rust
pub fn interpolate(samples: &[f32], position: f32, channels: u16) -> (f32, f32)
```

One entry point, branches on channel count inside:

```rust
pub fn interpolate_stereo(samples: &[f32], position: f32) -> (f32, f32)
pub fn interpolate_mono(samples: &[f32], position: f32) -> (f32, f32)
```

Stereo's interleaved (`L0 R0 L1 R1...`) so one interpolated pair needs 4 real reads, before and after for L, before and after for R. Mono isn't interleaved (`S0 S1 S2...`), only needs 2 reads, and that one value gets copied into both L and R on return. No panning happening here, that's its own feature, not built yet. This just mirrors mono into both channels for now.

Weighting's the same idea either way. `frac = position - position.floor()` is how far between the two samples you are. Earlier sample gets `(1 - frac)`, later gets `frac`. It's a cheap stand in for the real answer (sinc reconstruction, which looks at a bunch of surrounding samples instead of just 2), traded accuracy for speed. Good enough here, would be too much aliasing for something mastering grade.

## Frame Math — Mono vs Stereo

Three small helpers that only exist because stereo and mono have different frame widths in memory (2 slots vs 1), and that difference touches a bunch of math that used to just assume stereo everywhere.

```rust
pub fn align_to_frame(pos: f32) -> usize
```

Rounds position down to the nearest even index. Only makes sense for stereo since a frame always starts on an even slot. Mono doesn't need this at all, every index is a valid frame. Callers check channel count first and pick this or a plain floor.

```rust
pub fn frame_advance(channels: u16) -> f32
```

How far position moves per sample, before multiplying by playback_rate. 1.0 mono, 2.0 stereo. Used for advancing a voice and for converting step length into the same units position and stop_at_frame are compared in.

```rust
pub fn frame_lookahead(channels: u16) -> usize
```

How many extra samples past the current position need to actually exist in the buffer so interpolate doesn't read off the end. 1 for mono (pos, pos+1), 3 for stereo (pos through pos+3).

## Limiting — Keeping Output in Range

```rust
pub fn soft_clip(x: f32) -> f32
```

`x.tanh()`. Guarantees output never goes past ±1.0, which digital audio can't represent past anyway. Used tanh instead of a hard clamp because a hard clamp flattens the top of the waveform into a sharp corner once it crosses the ceiling, and that corner is literally what clipping sounds like. tanh rounds it off instead, quiet stuff passes through basically untouched, loud stuff gets squashed gradually instead of chopped flat.

Applied once per channel, after everything's already summed into the final mix. Not per voice, not after every individual addition, since clipping a partial sum and then stacking more on top of it just compounds distortion instead of actually limiting the real total.
