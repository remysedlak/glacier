# Signal Analysis

Functions in this chapter extract information *from* an existing buffer of
samples — loudness, frequency content, zero-crossing behavior. None of them
generate or alter audio; they measure it.

## Single-Window Measurements

Three windowed measurements share the same shape: take a slice of samples,
return one number describing that whole slice.

```rust
pub fn peak_window(samples: &[f32]) -> f32       // max(|x|)
pub fn rms_window(samples: &[f32]) -> f32        // sqrt(mean(x²))
pub fn zcr_window(samples: &[f32]) -> usize       // count of sign changes
```

- **`peak_window`** — the largest absolute sample value in the window. Used
  for peak-hold meters, where you want the loudest instant, not an average.
- **`rms_window`** — root-mean-square: `sqrt(Σx² / n)`. This is what "loudness"
  usually means perceptually, since it reflects energy over the window
  rather than a single spike.
- **`zcr_window`** — counts how many times the signal crosses zero within
  the window. Rises with pitch/noisiness; a pure low sine has a low ZCR, a
  cymbal hit has a high one. ([Reference](https://www.sciencedirect.com/topics/engineering/zero-crossing-rate))

## Time-Series Versions — Hop Over a Longer Buffer

`rms`, `peak`, and `zcr` are the windowed versions applied repeatedly across
a longer buffer, producing a time series instead of one number:

```rust
pub fn rms(samples: &[f32], window_size: usize, hop_size: usize) -> Vec<f32>
pub fn peak(samples: &[f32], window_size: usize, hop_size: usize) -> Vec<f32>
pub fn zcr(samples: &[f32], window_size: usize, hop_size: usize) -> Vec<usize>
```

Each slides a `window_size`-wide window across `samples`, advancing by
`hop_size` each step, and calls the matching `_window` function on each
slice. `hop_size < window_size` gives overlapping windows (smoother output,
more compute); `hop_size == window_size` gives non-overlapping windows.

## Envelope Follower — Smoothing a Time Series

```rust
pub fn envelope_follower(rms: &[f32], attack: f32, release: f32) -> Vec<f32>
```

Takes an RMS time series (typically from `rms()`) and smooths it with
different speeds depending on direction: `attack` coefficient when the
signal is rising, `release` when falling. This is the standard envelope-
follower shape from analog dynamics processing — fast attack lets transients
through quickly, slower release avoids the output chattering on every small
dip. Internally this is just repeated calls to `smooth_toward` (see
Synthesis chapter) with the coefficient chosen per-sample based on direction.

## Spectrum Analysis — FFT

### SpectrumAnalyzer
```rust
pub struct SpectrumAnalyzer { /* fft: Arc<dyn Fft<f32>>, window_size: usize */ }
impl SpectrumAnalyzer {
    pub fn new(window_size: usize) -> Self
    pub fn process(&self, samples: &[f32]) -> Vec<f32>
}
```
This is the one stateful type in the crate — constructing it plans the FFT
once (via `rustfft`'s `FftPlanner`), so repeated calls to `process` reuse
that plan rather than re-planning every call. `process` returns magnitudes
for bins `0` through Nyquist (`window_size / 2 + 1` bins) — the upper half
of a real-valued FFT is a mirror image and is discarded, since it carries no
additional information.

**Output is raw magnitude, not dB** — always pass it through
`magnitude_to_db` before displaying (see below); raw FFT magnitude scales
with `window_size` and has no intuitive relationship to perceived loudness.

### Windowing Before FFT — Hann Window
```rust
pub fn hann_window(samples: usize) -> Vec<f32>
pub fn window_compensation(window: &[f32]) -> f32
```
Running an FFT directly on a raw slice assumes that slice is exactly one
period of a periodic signal — it almost never is, and the discontinuity at
the slice's edges leaks energy across frequency bins ("spectral leakage").
Multiplying the slice by a Hann window tapers both edges toward zero first,
reducing that leakage at the cost of attenuating the signal's actual
amplitude — `window_compensation` computes the correction factor
(`window.len() / sum(window)`) to undo that attenuation, so switching window
size or type doesn't silently change the apparent loudness of the result.

### Converting Magnitude to Something Displayable
```rust
pub fn magnitude_to_db(magnitude: f32, window_size: usize, compensation: f32) -> f32
```
Normalizes a raw magnitude by `window_size` and the window compensation
factor, then converts to dB (`20 * log10(x)`), floored at a minimum (`1e-6`)
to avoid `log10(0)` producing `-inf` for silence.

### Bin ↔ Frequency Conversion
```rust
pub fn bin_to_freq(bin: usize, sample_rate: f32, window_size: usize) -> f32
pub fn freq_to_bin(freq: f32, sample_rate: f32, window_size: usize) -> usize
pub fn freq_resolution_per_bin(sample_rate: f32, window_size: usize) -> u32
```
An FFT bin index is meaningless without knowing the sample rate and window
size used to produce it — these convert between "bin N" and "this many Hz."
`freq_resolution_per_bin` is the Hz-per-bin spacing itself (how much
frequency one bin step represents), useful for deciding window size against
a desired frequency resolution.

### Log-Scaled Display Position
```rust
pub fn freq_to_log_position(freq: f32, min_freq: f32, max_freq: f32) -> f32
```
Maps a frequency to a `0.0..1.0` position on a *logarithmic* scale, not
linear. Pitch perception is logarithmic (each octave doubles frequency but
feels like an equal step), so mapping bins to screen pixels linearly crushes
almost all musically relevant content — bass through midrange — into a thin
sliver near the low end, with treble taking up most of the width for no
perceptual reason. Always use this instead of a raw bin-to-pixel mapping
when laying out a spectrum display.

### Short-Time Fourier Transform (STFT)
```rust
pub fn stft(samples: &[f32], window_size: usize, hop_size: usize) -> Vec<Vec<f32>>
```
Applies the Hann-window-then-FFT pipeline repeatedly across a buffer, hopping
by `hop_size` each time — one magnitude spectrum per hop, giving frequency
content *over time* rather than a single snapshot. Output is still raw
magnitude per frame; each inner `Vec<f32>` needs `magnitude_to_db` before
display, same as a single `SpectrumAnalyzer::process` call.
