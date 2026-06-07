/* dsp.rs
 *
 * mathematical functions to compute signals and samples
 */

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
