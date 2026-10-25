use crate::project::Track;
use glacier_dsp::{
    envelope::{flush_denormal, smooth_toward},
    resample::{align_to_frame, frame_advance, frame_lookahead, interpolate},
};

pub fn mix_voice(track: &mut Track, voice: usize, master_volume: f32, sample: &mut [f32]) {
    let channels = track.data.channels;
    let pos = if channels == 1 {
        track.voice_pool.positions[voice] as usize
    } else {
        align_to_frame(track.voice_pool.positions[voice])
    };
    let lookahead = frame_lookahead(channels);
    let stop = track.voice_pool.stop_at_frames[voice].unwrap_or(track.samples.len() as f32);

    if pos + lookahead >= stop.min(track.samples.len() as f32) as usize {
        track.voice_pool.active[voice] = false;
        return;
    }

    track.voice_pool.current_volumes[voice] = flush_denormal(smooth_toward(
        track.voice_pool.current_volumes[voice],
        track.voice_pool.target_volumes[voice],
        0.01,
    ));

    let (l, r) = interpolate(&track.samples, track.voice_pool.positions[voice], channels);
    let gain = track.voice_pool.current_volumes[voice] * track.data.track_volume * master_volume;
    sample[0] += l * gain;
    sample[1] += r * gain;

    let advance = frame_advance(channels);
    track.voice_pool.positions[voice] += advance * track.voice_pool.rates[voice];

    track.rms_l = flush_denormal(smooth_toward(track.rms_l, l * l, 0.01));
    track.rms_r = flush_denormal(smooth_toward(track.rms_r, r * r, 0.01));
    track.peak_hold = track.peak_hold.max(l.abs().max(r.abs()));
}
