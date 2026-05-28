/* dsp.rs
 *
 * mathematical functions to compute signals and samples
 */

/// calculate how many samples are in one step of audio
pub fn samples_per_step(sample_rate: f32, bpm: f32) -> f32 {
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
