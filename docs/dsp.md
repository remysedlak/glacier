# dsp.md — DSP Learning Journal (glacier)

A living document. Concepts section is reference; Journal is chronological scratch.

---

## Concepts

### Sample Rate & the Audio Callback

The sample rate is how many individual amplitude values are produced per second — typically 44100 Hz (CD quality) or 48000 Hz. CPAL asks your callback to fill a buffer of samples on demand; you don't control when it's called, only what you write into `data`.

Each element in `data` is one sample. For stereo, samples are interleaved: `[L, R, L, R, ...]`, so a buffer of 512 elements is 256 frames.

**In glacier:** `sample_rate_f` is pulled from `config.sample_rate` and passed into timing calculations. The callback fills `data.chunks_mut(2)` — each chunk is one stereo frame.

---

### BPM, Steps, and Timing

To know when to advance a step, you need to know how many samples fit in one step at the current BPM.

```
samples_per_step = sample_rate / (bpm / 60.0 * 4.0)
```

Breaking it down:
- `bpm / 60.0` → beats per second
- `* 4.0` → steps per beat (16th notes = 4 per beat in 4/4)
- `sample_rate / result` → samples per step

`sample_counter` accumulates `data.len() / 2` each callback (frames, not samples). When it exceeds `samples_per_step`, a step fires and resets to 0.

**Hard lesson:** `sample_counter` increments by `data.len() / 2` — the buffer is interleaved stereo so half the elements are frames. Getting this wrong makes timing drift or double-fire.

**Also:** initialize `current_step` to `max_steps - 1` so the first increment lands on step 0, not step 1. Otherwise the first beat is always skipped.

---

### Normalization

WAV files store integers. To convert to float audio in the range `[-1.0, 1.0]`:

```
sample_f32 = raw_integer / 2^(bits - 1)
```

For 16-bit audio: divide by `32768.0`. For 24-bit: divide by `8388608.0`.

**Hard lesson:** `reader.spec()` must be called before `reader.samples()` — spec gives you the bit depth needed for the divisor.

---

### Playback Rate & Pitch (Semitones)

Pitch and playback rate are directly related: playing a sample twice as fast raises pitch by one octave (12 semitones). Equal temperament divides the octave into 12 logarithmically equal steps, so the relationship is exponential.

```
playback_rate = 2.0 ^ (semitones / 12.0)
```

Where `semitones = pitch - root_note`. If pitch == root_note, rate = 1.0 (unchanged). Each semitone up multiplies rate by ~1.0595.

```rust
pub fn semitones_to_rate(pitch: u8, root_note: u8) -> f32 {
    2.0_f32.powf((pitch as f32 - root_note as f32) / 12.0)
}
```

`track.position += 2.0 * track.playback_rate` — the `2.0` accounts for stereo interleaving (advancing 2 array positions per frame).

---

### Volume Smoothing (Exponential Decay / One-pole Low-pass)

Snapping volume to a target instantly causes a click — a discontinuity in the waveform. The fix is to ease toward the target each sample:

```
current = current + (target - current) * coeff
```

With a small `coeff` (e.g. `0.01`), the value approaches the target exponentially — fast at first, then slower. This is a one-pole IIR low-pass filter, also called a leaky integrator.

```rust
pub fn smooth_toward(current: f32, target: f32, coeff: f32) -> f32 {
    current + (target - current) * coeff
}
```

`target_volume` is set on note trigger (from MIDI velocity), `current_volume` chases it each sample.

**Shutdown fade:** `shutdown_volume` decrements inside the sample loop, not per callback. Per-callback would be far too abrupt or slow depending on buffer size.

---

### Stereo Interleaving

CPAL buffers interleave channels: `[L0, R0, L1, R1, ...]`. Stereo WAVs from hound come out the same way.

This is why:
- `sample_counter += data.len() / 2` — half the elements are frames
- `track.position += 2.0 * rate` — advance by 2 to hit the next frame's L sample
- `track.samples[pos]` is L, `track.samples[pos + 1]` is R

Mixing is additive — each track's L and R contribution is summed into `sample[0]` and `sample[1]` before the frame is written.

---

## Journal

### 2026-05-28

Started the dsp.md. glacier's `dsp.rs` currently has three functions extracted from what used to live inline in `audio.rs`:

- `samples_per_step` — timing math for the step sequencer
- `smooth_toward` — exponential approach for volume ramping (anti-click)
- `semitones_to_rate` — pitch transposition via playback rate

The abstraction is good: `audio.rs` is a sequencer and mixer, not a math library.

Solid so far: why interleaved stereo means position advances by 2, why smoothing needs to happen per-sample not per-callback, the semitone-to-rate formula and why it's exponential.

Want to go deeper on: what makes `smooth_toward` a low-pass filter mathematically, Nyquist theorem, what aliasing actually sounds like in a sampler context.
