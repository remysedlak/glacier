use rustfft::{Fft, FftPlanner, num_complex::Complex32};

use std::{f32::consts::TAU, sync::Arc};

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

    /// Computes the FFT magnitude spectrum for a block of real-valued samples.
    /// Returns `window_size / 2 + 1` bins (the Nyquist-limited half of a real FFT).
    pub fn process(&self, samples: &[f32]) -> Vec<f32> {
        // converts every real sample into a complex number with an imaginary part of 0.0
        let mut buffer: Vec<Complex32> = samples.iter().map(|&x| Complex32::new(x, 0.0)).collect();

        // in place FFT
        self.fft.process(&mut buffer);

        // fft is symmetric
        buffer[..self.window_size / 2 + 1]
            .iter()
            // get magnitude and ditch phase
            .map(|c| c.norm())
            .collect()
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

#[cfg(test)]
mod spectral_test {
    use crate::{utils::sine_samples, window::window_compensation};

    use super::*;
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
}
