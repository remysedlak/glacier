use std::f32::consts::PI;

/// Soft-clips a sample into roughly [-1, 1] using tanh, avoiding a hard digital clip.
pub fn soft_clip(x: f32) -> f32 {
    x.tanh()
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

/// convert a pitch interval in semitones to a playback rate multiplier
pub fn semitones_to_rate(pitch: u8, root_note: u8) -> f32 {
    2.0_f32.powf((pitch as f32 - root_note as f32) / 12.0)
}

pub fn sine_samples() -> Vec<f32> {
    let mut samples: Vec<f32> = vec![];
    for i in 0..1024 {
        samples.push(f32::sin((i as f32 + 0.25) * (2.0 * PI / 1024.0)));
    }
    samples
}
