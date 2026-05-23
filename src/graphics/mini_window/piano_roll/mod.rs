use crate::graphics::{
    color::{BLACK, C_NOTE_COLOR, DARK_GRAY, LIGHT_GRAY, LL_GRAY, ORANGE, WHITE},
    widgets::Rectangle,
};

pub mod window;

pub const BLACK_SEMITONE_INDEXES: [u32; 5] = [1, 3, 5, 8, 10];
pub const SEMITONE_GAP: f32 = 18.0;
pub const SEMITONE_HEIGHT: f32 = 16.0;
pub const PIANO_ROLL_WIDTH: f32 = 64.0;
pub const OCTAVE_GAP: f32 = 216.0;
pub const PIANO_ROLL_MARGIN: f32 = 64.0;
pub const SEMITONE_OFFSET_X: f32 = 8.0;

pub struct PianoNote {
    pub pitch: u8,    // 0-127, maps to semitone+octave
    pub start: u32,   // which step column it starts on
    pub length: u32,  // how many step columns it spans
    pub velocity: u8, // 0-127
}

// return color for hover logic
pub fn black_piano_step_hover_color(rect: &Rectangle, mx: f32, my: f32) -> (f32, f32, f32) {
    if rect.is_hovered(mx, my) {
        DARK_GRAY
    } else {
        BLACK
    }
}
pub fn white_piano_step_hover_color(rect: &Rectangle, mx: f32, my: f32, index: u32) -> (f32, f32, f32) {
    if rect.is_hovered(mx, my) && index == 11 {
        ORANGE
    } else if index == 11 {
        C_NOTE_COLOR
    } else if rect.is_hovered(mx, my) {
        ORANGE
    } else {
        WHITE
    }
}
