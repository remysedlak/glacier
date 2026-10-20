use crate::{
    app::UiCommand,
    audio::{EngineState, SPECTRUM_WINDOW},
};

use glacier_dsp::{
    spectral::{hann_window, magnitude_to_db, SpectrumAnalyzer},
    window::window_compensation,
};
use ringbuf::traits::Producer;

pub fn push_meter_updates(
    engine_state: &mut EngineState,
    meter_counter: &mut usize,
    master_rms_l: f32,
    master_rms_r: f32,
    master_peak: &mut f32,
    samples_this_callback: usize,
) {
    *meter_counter += samples_this_callback;
    if *meter_counter < 1024 {
        return;
    }
    *meter_counter = 0;

    engine_state
        .audio_producer
        .try_push(UiCommand::MasterLevel(
            master_rms_l.sqrt(),
            master_rms_r.sqrt(),
            *master_peak,
        ))
        .ok();
    *master_peak = 0.0;

    for track in &mut engine_state.tracks {
        engine_state
            .audio_producer
            .try_push(UiCommand::TrackLevel(
                track.data.id,
                track.rms_l.sqrt(),
                track.rms_r.sqrt(),
                track.peak_hold,
            ))
            .ok();
        track.peak_hold = 0.0;
    }
}

/// Update the UI spectrum analyzer with new frame data
pub fn push_spectrum_frame(
    engine_state: &mut EngineState,
    spectrum_analyzer: &SpectrumAnalyzer,
    spectrum_buffer: &mut Vec<f32>,
) {
    // A spectrum analysis (FFT) needs a fixed-size block of samples
    if spectrum_buffer.len() < SPECTRUM_WINDOW {
        return;
    }

    // apply hann window and compensate
    let window = hann_window(SPECTRUM_WINDOW);

    // compensation is a correction factor that scales the result back up to compensate for that energy loss
    // ...otherwise dB readings would all read artificially quiet.
    let compensation = window_compensation(&window);

    // Applies the window: multiplies each sample by its corresponding window coefficient
    engine_state.windowed.clear();
    engine_state.windowed.extend(
        spectrum_buffer
            .iter()
            .zip(window.iter())
            .map(|(x, w)| x * w),
    );

    // This is where the actual FFT happens
    let magnitudes = spectrum_analyzer.process(&engine_state.windowed);

    // Converts magnitude to dB
    engine_state.db.clear();
    engine_state.db.extend(
        magnitudes
            .iter()
            .map(|m| magnitude_to_db(*m, SPECTRUM_WINDOW, compensation)),
    );

    // the full Vec gets handed off to the ringbuf, not dropped synchronously on the audio thread
    engine_state
        .audio_producer
        .try_push(UiCommand::SpectrumFrame(std::mem::take(
            &mut engine_state.db,
        )))
        .ok();

    // clear audio engine's spectrum db buffer
    spectrum_buffer.clear();
}
