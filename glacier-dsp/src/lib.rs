use std::f32::consts::TAU;

/// library contains implementations of ZCR, RMSE/Peaks, Envelope Follower

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

/// compute one discrete fourier transform
pub fn dft_window(samples: &[f32]) -> f32 {
    // X[k] = Σ x[n] * e^(-j2πkn/N)
    let mut sum = 0.0;
    for sample in samples {
        sum += sample * f32::exp(3.0).powf(-TAU * sample / samples.len() as f32)
    }
    sum
}

/// Hann window samples. used for smoothing non-periodic captured signals
/// * narrows the frequency spectrum from an FFT
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

pub fn freq_resolution_per_bin(sample_rate: f32, window_size: usize) -> u32 {
    sample_rate as u32 / window_size as u32
}

// Short-time Fourier transform
pub fn stft() {}

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
    pub fn sine_samples() -> Vec<f32> {
        let mut samples: Vec<f32> = vec![];
        for i in 0..1024 {
            samples.push(f32::sin((i as f32 + 0.25) * (2.0 * PI / 1024.0)));
        }
        samples
    }
}
