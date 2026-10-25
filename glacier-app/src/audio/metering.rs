use crate::{
    app::UiCommand,
    audio::{EngineState, SPECTRUM_BINS, SPECTRUM_WINDOW},
};

use glacier_dsp::spectral::{magnitude_to_db, SpectrumAnalyzer};
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
                track.id,
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
    spectrum_analyzer: &mut SpectrumAnalyzer,
    hann_window: &[f32],
    compensation: f32,
    spectrum_buffer: &[f32],
    spectrum_db: &mut [f32; SPECTRUM_BINS],
) {
    // A spectrum analysis (FFT) needs a fixed-size block of samples
    if spectrum_buffer.len() != SPECTRUM_WINDOW {
        return;
    }

    // The analyzer owns its preallocated FFT input and scratch buffers. Keep
    // the display data in a fixed-size array so this path neither allocates nor
    // transfers heap ownership to the UI thread.
    spectrum_analyzer.process_windowed_into(spectrum_buffer, hann_window, spectrum_db);
    for magnitude in spectrum_db.iter_mut() {
        *magnitude = magnitude_to_db(*magnitude, SPECTRUM_WINDOW, compensation);
    }

    // The array is copied into the bounded ring. A full ring drops only this
    // fixed-size value, never a heap allocation on the audio thread.
    engine_state.spectrum_producer.try_push(*spectrum_db).ok();
}
