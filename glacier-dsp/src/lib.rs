/// library contains implementations of ZCR, RMSE/Peaks,

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
        if (sample[0] > 0.0 && sample[1] < 0.0 || sample[0] < 0.0 && sample[1] > 0.0) && (sample[0] - sample[1]).abs() > 0.0 {
            crosses += 1;
        }
    }
    crosses
}

/// Root-Mean Square Energy: used for volume tracking over time (db meter)
/// # Arguments
/// * hop_size - how far apart each snapshot is taken
/// * window_size - how wide each snapshot is
pub fn rms(samples: &[f32], window_size: usize, hop_size: usize) -> Vec<f32> {
    let mut rms_vector: Vec<f32> = vec![];
    for hop in 0..(samples.len() / hop_size) - 1 {
        rms_vector.push(rms_window(&samples[(hop * hop_size)..(hop * hop_size + window_size)]));
    }
    rms_vector
}

/// Peak ampltidue values
pub fn peak(samples: &[f32], window_size: usize, hop_size: usize) -> Vec<f32> {
    let mut peak_vector: Vec<f32> = vec![];
    for hop in 0..(samples.len() / hop_size) - 1 {
        peak_vector.push(peak_window(&samples[(hop * hop_size)..(hop * hop_size + window_size)]));
    }
    peak_vector
}

/// Zero Crossing Rate
pub fn zcr(samples: &[f32], window_size: usize, hop_size: usize) -> Vec<usize> {
    let mut zcr_vector: Vec<usize> = vec![];
    for hop in 0..(samples.len() / hop_size) - 1 {
        zcr_vector.push(zcr_window(&samples[(hop * hop_size)..(hop * hop_size + window_size)]));
    }
    zcr_vector
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
    pub fn sine_samples() -> Vec<f32> {
        let mut samples: Vec<f32> = vec![];
        for i in 0..1024 {
            samples.push(f32::sin((i as f32 + 0.25) * (2.0 * PI / 1024.0)));
        }
        samples
    }
}
