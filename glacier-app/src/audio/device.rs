use cpal::traits::DeviceTrait;
use cpal::{SampleFormat, StreamConfig};

/// Attempts to build a real (short-lived, immediately-dropped) output stream
/// against `cfg` using a no-op callback, purely to discover whether the
/// underlying driver (ALSA/hw_params, WASAPI, CoreAudio, etc.) actually
/// accepts this exact channels+rate+format combination on THIS machine.
/// Returns true if it does. This is the only reliable way to know — reported
/// "supported config ranges" can include values that fail validation anyway.
fn probes_ok(device: &cpal::Device, cfg: &StreamConfig, sample_format: SampleFormat) -> bool {
    let err_fn = |_err| {};
    let result = match sample_format {
        SampleFormat::F32 => device
            .build_output_stream(cfg, move |_: &mut [f32], _: &_| {}, err_fn, None)
            .map(|_stream| ()),
        SampleFormat::I16 => device
            .build_output_stream(cfg, move |_: &mut [i16], _: &_| {}, err_fn, None)
            .map(|_stream| ()),
        SampleFormat::U16 => device
            .build_output_stream(cfg, move |_: &mut [u16], _: &_| {}, err_fn, None)
            .map(|_stream| ()),
        _ => return false, // unsupported format, don't even try
    };
    result.is_ok()
}

/// Discovers a StreamConfig + SampleFormat combination that this device
/// actually accepts, starting from cpal's reported default and falling back
/// through common alternatives if the default fails validation. Never
/// assumes a fixed rate/format — probes reality on whatever machine this
/// runs on.
pub fn find_working_config(
    device: &cpal::Device,
    default_config: &cpal::SupportedStreamConfig,
) -> (StreamConfig, SampleFormat) {
    let default_channels = default_config.channels();
    let default_rate = default_config.sample_rate();
    let default_format = default_config.sample_format();

    // Try the reported default combo first.
    let default_cfg = StreamConfig {
        channels: default_channels,
        sample_rate: default_rate,
        buffer_size: cpal::BufferSize::Default,
    };
    if probes_ok(device, &default_cfg, default_format) {
        return (default_cfg, default_format);
    }

    // Fall back: try common rates/formats/channel counts in order of
    // likelihood, stopping at the first one that actually validates.
    let candidate_rates = [default_rate, 48000, 44100, 96000, 22050];
    let candidate_formats = [default_format, SampleFormat::F32, SampleFormat::I16];
    let candidate_channels = [default_channels, 2, 1];

    for &channels in &candidate_channels {
        for &rate in &candidate_rates {
            for &format in &candidate_formats {
                let cfg = StreamConfig {
                    channels,
                    sample_rate: rate,
                    buffer_size: cpal::BufferSize::Default,
                };
                if probes_ok(device, &cfg, format) {
                    return (cfg, format);
                }
            }
        }
    }

    panic!(
        "No working audio output configuration found for this device. \
         Reported default was {default_channels} channel(s), {default_rate}Hz, {default_format:?}, \
         but none of the tested channel/rate/format combinations were accepted by the driver."
    );
}
