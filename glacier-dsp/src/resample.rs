/// even out a sample position to force stereo operations on even array
pub fn align_to_frame(pos: f32) -> usize {
    (pos as usize) & !1
}

/// Linearly interpolate a stereo-interleaved sample pair at a fractional position.
/// Returns (left, right). `pos` must leave room for 4 more values in `samples`.
pub fn interpolate_stereo(samples: &[f32], position: f32) -> (f32, f32) {
    let pos = align_to_frame(position); // align_to_frame, inlined or kept separate
    let frac = position - position.floor();
    let l = samples[pos] + frac * (samples[pos + 2] - samples[pos]);
    let r = samples[pos + 1] + frac * (samples[pos + 3] - samples[pos + 1]);
    (l, r)
}

/// Linearly interpolate a mono sample stream at a fractional position.
/// Returns the same value duplicated for (left, right).
/// `pos` must leave room for 1 more value in `samples`.
pub fn interpolate_mono(samples: &[f32], position: f32) -> (f32, f32) {
    let pos = position as usize;
    let frac = position - position.floor();
    let value = samples[pos] + frac * (samples[pos + 1] - samples[pos]);
    (value, value)
}

/// helper method to match interpolation method to channel count
pub fn interpolate(samples: &[f32], position: f32, channels: u16) -> (f32, f32) {
    if channels == 1 {
        interpolate_mono(samples, position)
    } else {
        interpolate_stereo(samples, position)
    }
}

// in glacier_dsp
/// How many additional samples past `pos` must exist in the buffer for a
/// safe interpolation read, given the source's channel count.
pub fn frame_lookahead(channels: u16) -> usize {
    if channels == 1 { 1 } else { 3 }
}

/// How far position should advance per sample for one frame's worth of
/// playback, given the source's channel count. Multiply by playback rate.
pub fn frame_advance(channels: u16) -> f32 {
    if channels == 1 { 1.0 } else { 2.0 }
}
