//! library contains implementations of ZCR, RMSE/Peaks, Envelope Follower

use rustfft::{Fft, FftPlanner, num_complex::Complex32};
use std::f32::consts::TAU;
use std::sync::Arc;

pub struct SpectrumAnalyzer {
    fft: Arc<dyn Fft<f32>>,
    window_size: usize,
}

impl SpectrumAnalyzer {
    pub fn new(window_size: usize) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(window_size);
        Self { fft, window_size }
    }

    pub fn process(&self, samples: &[f32]) -> Vec<f32> {
        let mut buffer: Vec<Complex32> = samples.iter().map(|&x| Complex32::new(x, 0.0)).collect();
        self.fft.process(&mut buffer);
        buffer[..self.window_size / 2 + 1]
            .iter()
            .map(|c| c.norm())
            .collect()
    }
}

/// Helper. returns a peak (maximum) amplitude for one window
///
/// max(Σ |x|)
/// # Arguments
/// * samples - slice of amplitude values
pub fn peak_window(samples: &[f32]) -> f32 {
    let mut peak: f32 = 0.0; // accumalte the square of each sample's ampltiude
    for sample in samples {
        peak = f32::max(f32::abs(*sample), peak);
    }
    peak
}

/// Helper. returns a root mean square for one window
///
/// sqrt(Σx^2 / n)
/// # Arguments
/// * samples - slice of amplitude values
pub fn rms_window(samples: &[f32]) -> f32 {
    let mut sum = 0.0; // accumalte the square of each sample's ampltiude
    for sample in samples {
        sum += sample * sample;
    }
    let mean = sum / samples.len() as f32; // get the mean value
    let rms = mean.powf(0.5); // take the square root
    rms
}

/// helper. returns the amount of zero crosses for one window
/// * https://www.sciencedirect.com/topics/engineering/zero-crossing-rate
pub fn zcr_window(samples: &[f32]) -> usize {
    let mut crosses = 0; // accumalte the square of each sample's ampltiude
    for sample in samples.windows(2) {
        // amplitude crosses zero (+ <-> -)
        if (sample[0] > 0.0 && sample[1] < 0.0 || sample[0] < 0.0 && sample[1] > 0.0)
            && (sample[0] - sample[1]).abs() > 0.0
        {
            crosses += 1;
        }
    }
    crosses
}

/// Hann window samples. used for smoothing non-periodic captured signals
/// * narrows the frequency spectrum leakage from FT
pub fn hann_window(samples: usize) -> Vec<f32> {
    let mut freq: Vec<f32> = vec![];
    let n = samples as f32;
    for sample in 0..samples {
        // 0.5 * (1 - cos(2πn/N))
        let windowed_sample = 0.5 * (1.0 - f32::cos(TAU * sample as f32 / n));
        freq.push(windowed_sample)
    }
    freq
}

/// Amplitude correction factor for a window function, to undo the attenuation
/// a window applies before analysis. Multiply raw FFT magnitudes by this
/// (see `magnitude_to_db`) so switching window sizes or window types doesn't
/// change the apparent loudness of the spectrum.
///
/// # Arguments
/// * window - the window samples (e.g. output of `hann_window`)
pub fn window_compensation(window: &[f32]) -> f32 {
    let sum: f32 = window.iter().sum();
    if sum == 0.0 {
        1.0
    } else {
        window.len() as f32 / sum
    }
}

/// Convert one raw FFT magnitude into dB, correcting for FFT size and
/// (optionally) the amplitude loss from a window function.
///
/// # Arguments
/// * magnitude - one bin's value from `SpectrumAnalyzer::process`
/// * window_size - N, the size of the window the FFT was run on
/// * compensation - output of `window_compensation`; pass 1.0 if no window was applied
pub fn magnitude_to_db(magnitude: f32, window_size: usize, compensation: f32) -> f32 {
    let normalized = magnitude * compensation / window_size as f32;
    20.0 * normalized.max(1e-6).log10()
}

/// Convert a bin index from an FFT of size `window_size` into the frequency
/// (Hz) that bin represents.
pub fn bin_to_freq(bin: usize, sample_rate: f32, window_size: usize) -> f32 {
    bin as f32 * sample_rate / window_size as f32
}

/// Inverse of `bin_to_freq`: convert a frequency (Hz) into the nearest bin
/// index for an FFT of size `window_size`.
pub fn freq_to_bin(freq: f32, sample_rate: f32, window_size: usize) -> usize {
    ((freq * window_size as f32 / sample_rate).round() as usize).min(window_size / 2)
}

/// Map a frequency to a 0.0–1.0 position on a logarithmic scale between
/// `min_freq` and `max_freq`. Use this instead of a raw bin index when laying
/// out a spectrum display, since pitch perception is logarithmic and a linear
/// bin-to-pixel mapping crushes all musically relevant content into a sliver
/// on the low end.
///
/// # Arguments
/// * freq - frequency in Hz, e.g. from `bin_to_freq`
/// * min_freq - lower bound of the display range (e.g. 20.0)
/// * max_freq - upper bound of the display range (e.g. sample_rate / 2.0)
pub fn freq_to_log_position(freq: f32, min_freq: f32, max_freq: f32) -> f32 {
    let freq = freq.max(min_freq);
    (freq.log2() - min_freq.log2()) / (max_freq.log2() - min_freq.log2())
}

pub fn freq_resolution_per_bin(sample_rate: f32, window_size: usize) -> u32 {
    sample_rate as u32 / window_size as u32
}

/// Short-time Fourier transform: windowed + hopped FFT over a buffer
/// returns one magnitude spectrum per frame. Magnitudes here are still raw
/// FFT output — pass them through `magnitude_to_db` (with
/// `window_compensation(&hann_window(window_size))`) before displaying.
pub fn stft(samples: &[f32], window_size: usize, hop_size: usize) -> Vec<Vec<f32>> {
    let window = hann_window(window_size);
    let mut frames: Vec<Vec<f32>> = vec![];
    let spectrum_analyzer = SpectrumAnalyzer::new(window_size);
    let mut pos = 0;
    while pos + window_size <= samples.len() {
        let windowed: Vec<f32> = samples[pos..pos + window_size]
            .iter()
            .zip(window.iter())
            .map(|(x, w)| x * w)
            .collect();
        frames.push(spectrum_analyzer.process(&windowed));
        pos += hop_size;
    }
    frames
}

/// Root-Mean Square Energy: used for volume tracking over time (db meter)
pub fn rms(samples: &[f32], window_size: usize, hop_size: usize) -> Vec<f32> {
    let mut rms_vector: Vec<f32> = vec![];
    for hop in 0..(samples.len() / hop_size) - 1 {
        rms_vector.push(rms_window(
            &samples[(hop * hop_size)..(hop * hop_size + window_size)],
        ));
    }
    rms_vector
}

/// Peak ampltidue values
pub fn peak(samples: &[f32], window_size: usize, hop_size: usize) -> Vec<f32> {
    let mut peak_vector: Vec<f32> = vec![];
    for hop in 0..(samples.len() / hop_size) - 1 {
        peak_vector.push(peak_window(
            &samples[(hop * hop_size)..(hop * hop_size + window_size)],
        ));
    }
    peak_vector
}

/// Zero Crossing Rate
pub fn zcr(samples: &[f32], window_size: usize, hop_size: usize) -> Vec<usize> {
    let mut zcr_vector: Vec<usize> = vec![];
    for hop in 0..(samples.len() / hop_size) - 1 {
        zcr_vector.push(zcr_window(
            &samples[(hop * hop_size)..(hop * hop_size + window_size)],
        ));
    }
    zcr_vector
}

/// Envelope Follower. smooths RMSE values over time
pub fn envelope_follower(rms: &[f32], attack: f32, release: f32) -> Vec<f32> {
    let mut smooth_vector: Vec<f32> = vec![];
    let mut previous = 0.0;
    for rms_value in rms.iter() {
        let coefficient = if *rms_value > previous {
            attack
        } else {
            release
        };
        let smoothed_rms = smooth_toward(previous, *rms_value, coefficient);
        smooth_vector.push(smoothed_rms);
        previous = smoothed_rms;
    }
    smooth_vector
}

pub fn samples_per_step(sample_rate: f32, bpm: f32) -> f32 {
    /*
     * Calculating samples_per_step:
     *
     * sample_rate: samples per second (HZ)
     * bpm: beats per minute
     *
     * There are 60 seconds in one minute -> beats_per_second = bpm / 60.0
     * There are 4 steps in one beat -> steps_per_second = bps * 4.0
     *
     * samples_per_second / steps_per_second is equivalent to samples / steps
     */

    sample_rate / (bpm / 60.0 * 4.0)
}

/// exponential decay
pub fn smooth_toward(current: f32, target: f32, coeff: f32) -> f32 {
    current + (target - current) * coeff
}

/// convert a pitch interval in semitones to a playback rate multiplier
pub fn semitones_to_rate(pitch: u8, root_note: u8) -> f32 {
    2.0_f32.powf((pitch as f32 - root_note as f32) / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;
    #[test]
    fn zero_rms_window() {
        let samples = &[0.0; 1024];
        let result: f32 = rms_window(samples);
        assert_eq!(result, 0.0);
    }
    #[test]
    fn sine_rms_window() {
        let samples = &sine_samples();
        let result: f32 = rms_window(samples);
        assert!((result - 0.7071).abs() < 0.0001);
    }
    #[test]
    fn sine_peak_window() {
        let samples = &sine_samples();
        let result: f32 = peak_window(samples);
        assert!((result - 1.0).abs() < 0.0001);
    }
    #[test]
    fn sine_period_zcr() {
        let samples = &sine_samples();
        let result: usize = zcr_window(samples);
        assert_eq!(result, 1);
    }
    #[test]
    fn zero_rms() {
        let zero_samples: Vec<f32> = [0.0_f32; 8192].to_vec();
        let result: Vec<f32> = rms(&zero_samples, 1024, 512);
        let answer = [0.0_f32; 15];
        assert_eq!(result, answer);
    }
    #[test]
    fn zero_envelope_follower() {
        let zero_samples: Vec<f32> = [0.0_f32; 8192].to_vec();
        let result: Vec<f32> = rms(&zero_samples, 1024, 512);
        let envelope = envelope_follower(&result, 1.0, 0.01);
        let answer = [0.0_f32; 15];
        assert_eq!(envelope, answer);
    }
    #[test]
    fn stft_frame_count_and_bin_count() {
        let samples = &sine_samples(); // 1024 samples
        let frames = stft(samples, 1024, 512);
        assert_eq!(frames.len(), 1); // only one full 1024-window fits in 1024 samples
        assert_eq!(frames[0].len(), 1024 / 2 + 1); // bins up to Nyquist
    }
    #[test]
    fn hann_window_compensation_is_about_two() {
        let window = hann_window(1024);
        let compensation = window_compensation(&window);
        // Hann window averages ~0.5, so compensation should be ~2.0
        assert!((compensation - 2.0).abs() < 0.01);
    }
    #[test]
    fn magnitude_to_db_of_silence_floors_at_minimum() {
        let db = magnitude_to_db(0.0, 1024, 1.0);
        assert!((db - (-120.0)).abs() < 1.0);
    }
    #[test]
    fn bin_to_freq_matches_resolution() {
        // bin 1 at 44100 Hz / 1024 window should equal one resolution step
        let freq = bin_to_freq(1, 44100.0, 1024);
        let resolution = freq_resolution_per_bin(44100.0, 1024) as f32;
        assert!((freq - resolution).abs() < 1.0);
    }
    #[test]
    fn freq_to_log_position_bounds() {
        let low = freq_to_log_position(20.0, 20.0, 20000.0);
        let high = freq_to_log_position(20000.0, 20.0, 20000.0);
        assert!((low - 0.0).abs() < 0.0001);
        assert!((high - 1.0).abs() < 0.0001);
    }
    #[test]
    fn freq_to_bin_matches_bin_to_freq() {
        let freq = bin_to_freq(10, 44100.0, 1024);
        let bin = freq_to_bin(freq, 44100.0, 1024);
        assert_eq!(bin, 10);
    }
    pub fn sine_samples() -> Vec<f32> {
        let mut samples: Vec<f32> = vec![];
        for i in 0..1024 {
            samples.push(f32::sin((i as f32 + 0.25) * (2.0 * PI / 1024.0)));
        }
        samples
    }
}
