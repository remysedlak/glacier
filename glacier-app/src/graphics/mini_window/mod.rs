//! Mini windows are internal floating windows that use painters algorithm for visual hierarchy

use crate::graphics::TITLEBAR_HEIGHT;

pub mod mixer;
pub mod piano_roll;
pub mod playlist;
pub mod sequencer;
pub mod track;

pub const SEQUENCER_ID: usize = 0;
pub const PLAYLIST_ID: usize = 1;
pub const MIXER_ID: usize = 2;
pub const PIANO_ROLL_ID: usize = 3;

#[derive(Debug, PartialEq)]
/// Different types of MiniWindows
pub enum WindowKind {
    Sequencer,
    Playlist,
    Mixer,
    PianoRoll,
    TrackDetail(usize), // which track
}

#[derive(Debug)]
/// The MiniWindow is a internal draggable window that follows painters algorithm and culls or scissor rects overflowing shapes.
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
    pub fn new(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        title: &str,
        window_kind: WindowKind,
        is_open: bool,
    ) -> Self {
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
    /// if the mouse cursor is on top of a mindow. Used for bringing the window to the front after clicking an inactive window.
    pub fn is_hovered(&self, mouse_x: f32, mouse_y: f32) -> bool {
        mouse_x > self.x
            && mouse_x < self.x + self.width
            && mouse_y > self.y - TITLEBAR_HEIGHT
            && mouse_y < self.y + self.height
    }
}
