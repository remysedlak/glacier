//! file for drawing track sliders for volume control
use crate::graphics::color::{BLACK, LIGHT_GRAY};
use crate::graphics::{Rectangle, ScreenConfig, Vertex, NO_RADIUS};

pub const MIXER_TRACK_HEIGHT: f32 = 164.0;
pub const MIXER_TRACK_WIDTH: f32 = 4.0;
pub const THUMB_HEIGHT: f32 = 16.0;
pub const MIXER_THUMB_WIDTH: f32 = 32.0;

/// return the y position of the slider based on volume
fn volume_to_slider_position(volume: f32) -> f32 {
    (1.0 - volume) * MIXER_TRACK_HEIGHT
}

pub fn slider_y_origin(window_y: f32, window_height: f32) -> f32 {
    window_y + (window_height - crate::graphics::primitives::PAD_32) - 172.0
        + crate::graphics::primitives::PAD_16
        - crate::graphics::primitives::PAD_8
}

/// draw one slider for mixer.rs
pub fn draw(
    master_volume: f32,
    x: f32,
    y: f32,
    screen_config: &ScreenConfig,
    out: &mut Vec<Vertex>,
) {
    // TRACK (static)
    let track = Rectangle {
        x: x + (MIXER_THUMB_WIDTH / 2.0), // the track is in the midle of the button
        y,
        width: MIXER_TRACK_WIDTH,
        height: MIXER_TRACK_HEIGHT,
    };
    track.draw(screen_config, BLACK, NO_RADIUS, out);

    // THUMB (movable)
    let thumb = Rectangle {
        x,
        y: volume_to_slider_position(master_volume) + y,
        width: MIXER_THUMB_WIDTH,
        height: THUMB_HEIGHT,
    };
    thumb.draw(screen_config, LIGHT_GRAY, NO_RADIUS, out);
}
