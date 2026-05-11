use crate::graphics::widgets::TITLEBAR_HEIGHT;

pub const ONE_MEGABYTE: u64 = 1024 * 1024;
pub const BAR_GAP: f32 = 12.0;
pub const BUTTON_GAP: f32 = 24.0;
pub const TRACK_GAP: f32 = 72.0;

pub const KNOB_RADIUS: f32 = 13.0;

pub const MUTE_SQUARE_LENGTH: f32 = 12.0;

// padding constants
pub const PAD_64: f32 = 64.0;
pub const PAD_32: f32 = 32.0;
pub const PAD_16: f32 = 16.0;
pub const PAD_8: f32 = 8.0;
pub const PAD_4: f32 = 4.0;

#[derive(Debug)]
pub enum WindowKind {
    Sequencer,
    Playlist,
    Mixer,
    // PianoRoll,
    // InstrumentDetail(usize), // which instrument
}

pub struct MouseState {
    pub x: f32,
    pub y: f32,
    pub left_clicked: bool,
    pub right_clicked: bool,
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub shift_pressed: bool,
}

pub struct MiniWindow {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub title: String,
    pub is_open: bool,
    pub window_kind: WindowKind,
}
impl MiniWindow {
    /// Creates a movable new window
    pub fn new(x: f32, y: f32, width: f32, height: f32, title: &str, window_kind: WindowKind, is_open: bool) -> Self {
        Self {
            x,
            y,
            width,
            height,
            title: title.to_string(),
            is_open,
            window_kind,
        }
    }
    pub fn is_hovered(&self, mouse_x: f32, mouse_y: f32) -> bool {
        mouse_x > self.x && mouse_x < self.x + self.width && mouse_y > self.y - TITLEBAR_HEIGHT && mouse_y < self.y + self.height
    }
}

pub struct WindowDrawRange {
    pub vert_start: u32,
    pub vert_end: u32,
    pub char_start: usize,
    pub char_end: usize,
}
pub struct PlaylistDrawRanges {
    pub static_range: WindowDrawRange,
    pub header_range: WindowDrawRange,
    pub timeline_range: WindowDrawRange,
}

pub struct ScreenConfig {
    pub width: u32,
    pub height: u32,
}
