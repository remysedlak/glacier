//! Draws the power spectrum of the mixed audio signal as a log-scaled bar chart.
use crate::graphics::{
    color::{BLACK, ORANGE},
    Rectangle, ScreenConfig, Vertex, ICON_SIZE, NO_RADIUS, PLAY_Y_ORIGIN, RADIUS_4,
};

const MIN_FREQ: f32 = 20.0;
const DB_FLOOR: f32 = -60.0;
const DB_CEIL: f32 = 0.0;

/// draw power spectrum of song by taking a fourier series and displaying db log info
///
/// # Arguments
/// * spectrum_db - per-bin magnitude in dB, e.g. from `Graphics::spectrum`
/// * sample_rate - the audio device's sample rate (Hz)
/// * window_size - the FFT window size used to produce `spectrum_db`
pub fn draw(
    screen_config: &ScreenConfig,
    spectrum_db: &[f32],
    sample_rate: f32,
    window_size: usize,
    out: &mut Vec<Vertex>,
) {
    let spectrum_background = Rectangle {
        x: screen_config.width as f32 - 420.0,
        y: PLAY_Y_ORIGIN,
        width: ICON_SIZE * 5.0,
        height: ICON_SIZE,
    };
    spectrum_background.draw(screen_config, BLACK, RADIUS_4, out);

    if spectrum_db.is_empty() {
        return;
    }

    let max_freq = sample_rate / 2.0; // Nyquist
    let num_columns = spectrum_background.width.max(1.0) as usize;

    for column in 0..num_columns {
        // map this pixel column to the frequency range it represents on a log scale
        let ratio_start = column as f32 / num_columns as f32;
        let ratio_end = (column + 1) as f32 / num_columns as f32;
        let freq_start = log_position_to_freq(ratio_start, MIN_FREQ, max_freq);
        let freq_end = log_position_to_freq(ratio_end, MIN_FREQ, max_freq);

        let bin_start = glacier_dsp::freq_to_bin(freq_start, sample_rate, window_size);
        let bin_end = glacier_dsp::freq_to_bin(freq_end, sample_rate, window_size)
            .max(bin_start + 1)
            .min(spectrum_db.len());

        // take the loudest bin in this column's range; high columns cover many
        // bins each, and averaging them would flatten peaks that matter more
        // than the noise floor between them
        let db = spectrum_db[bin_start.min(spectrum_db.len().saturating_sub(1))..bin_end]
            .iter()
            .cloned()
            .fold(DB_FLOOR, f32::max);

        let normalized = ((db - DB_FLOOR) / (DB_CEIL - DB_FLOOR)).clamp(0.0, 1.0);
        let bar_height = normalized * spectrum_background.height;
        if bar_height <= 0.0 {
            continue;
        }

        let bar = Rectangle {
            x: spectrum_background.x + column as f32,
            y: spectrum_background.y + spectrum_background.height - bar_height,
            width: 1.0,
            height: bar_height,
        };
        bar.draw(screen_config, ORANGE, NO_RADIUS, out);
    }
}

/// Inverse of `glacier_dsp::freq_to_log_position`: given a 0.0–1.0 position on
/// a log scale between `min_freq` and `max_freq`, return the frequency there.
fn log_position_to_freq(position: f32, min_freq: f32, max_freq: f32) -> f32 {
    let log_min = min_freq.log2();
    let log_max = max_freq.log2();
    2f32.powf(log_min + position * (log_max - log_min))
}
