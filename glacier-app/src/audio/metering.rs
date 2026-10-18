use crate::{
    app::UiCommand,
    audio::{EngineState, SPECTRUM_WINDOW},
};
use glacier_dsp::{hann_window, magnitude_to_db, window_compensation, SpectrumAnalyzer};
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

pub fn push_spectrum_frame(
    engine_state: &mut EngineState,
    spectrum_analyzer: &SpectrumAnalyzer,
    spectrum_buffer: &mut Vec<f32>,
) {
    if spectrum_buffer.len() < SPECTRUM_WINDOW {
        return;
    }

    let window = hann_window(SPECTRUM_WINDOW);
    let compensation = window_compensation(&window);

    engine_state.windowed.clear();
    engine_state.windowed.extend(
        spectrum_buffer
            .iter()
            .zip(window.iter())
            .map(|(x, w)| x * w),
    );

    let magnitudes = spectrum_analyzer.process(&engine_state.windowed);
    engine_state.db.clear();
    engine_state.db.extend(
        magnitudes
            .iter()
            .map(|m| magnitude_to_db(*m, SPECTRUM_WINDOW, compensation)),
    );

    engine_state
        .audio_producer
        .try_push(UiCommand::SpectrumFrame(std::mem::take(
            &mut engine_state.db,
        )))
        .ok();

    spectrum_buffer.clear();
}
