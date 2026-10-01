pub mod window;
use crate::graphics::{color::Color, geometry::Rectangle, primitives::ScreenConfig};

pub const BLACK_SEMITONE_INDEXES: [u32; 5] = [1, 3, 5, 8, 10];
pub const SEMITONE_GAP: f32 = 18.0;
pub const SEMITONE_HEIGHT: f32 = 16.0;
pub const PIANO_ROLL_WIDTH: f32 = 64.0;
pub const OCTAVE_GAP: f32 = 216.0;
pub const PIANO_ROLL_MARGIN: f32 = 64.0;
pub const SEMITONE_OFFSET_X: f32 = 8.0;
pub const PIANO_ROLL_DEFAULT_Y: f32 = 1015.0;

/// The piano tray should show hover color or show active if its currently being played in the song.
pub fn white_piano_step_hover_color(
    rect: &Rectangle,
    mx: f32,
    my: f32,
    index: u32,
    screen_config: &ScreenConfig,
) -> Color {
    if rect.is_hovered(mx, my) && index == 11 {
        screen_config.palette.primary.shade(5)
    } else if index == 11 {
        screen_config.palette.primary.shade(6)
    } else if rect.is_hovered(mx, my) {
        screen_config.palette.primary.shade(5)
    } else {
        screen_config.palette.neutral.shade(7)
    }
}
