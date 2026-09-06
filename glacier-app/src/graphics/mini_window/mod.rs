//! Mini windows are internal floating windows that use painters algorithm for visual hierarchy

use winit::window::CursorIcon;

use crate::{
    app::{click::ClickResult, MouseState},
    graphics::{
        color::{DARK_GRAY, LIGHT_GRAY, WHITE},
        font::{TextItem, ROBOTO},
        geometry::Rectangle,
        primitives::{ScreenConfig, Vertex, NO_RADIUS, PAD_16, PAD_4, PAD_8},
    },
    project::TrackID,
};

pub const TOP_RADIUS_6: [f32; 4] = [6.0, 0.0, 6.0, 0.0];

pub const TITLEBAR_HEIGHT: f32 = 32.0;

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
    TrackDetail(TrackID), // which track
}

pub struct InteractionResult {
    pub click: ClickResult,
    pub cursor: CursorIcon,
}

impl InteractionResult {
    pub fn or(self, other: InteractionResult) -> InteractionResult {
        InteractionResult {
            click: if self.click != ClickResult::None {
                self.click
            } else {
                other.click
            },
            cursor: if self.cursor != CursorIcon::Default {
                self.cursor
            } else {
                other.cursor
            },
        }
    }
}
impl Default for InteractionResult {
    fn default() -> InteractionResult {
        InteractionResult {
            click: ClickResult::None,
            cursor: CursorIcon::Default,
        }
    }
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
    /// Returns the x position of where text will appear centered for a MiniWindow title bar
    pub fn center_title_x(&self, text: &str) -> f32 {
        let center_x = self.x + (self.width / 2.0);
        center_x - (text.len() as f32 * 9.0 / 2.0)
    }
    pub fn background(&self) -> Rectangle {
        Rectangle {
            x: self.x,
            y: self.y - 1.0,
            width: self.width,
            height: self.height,
        }
    }
    /// Draws the title bar of a MiniWindow, bordered so it meets up with the
    /// body's own border at the seam and reads as one outlined window.
    pub fn title_bar(
        &self,
        title: &str,
        screen_config: &ScreenConfig,
        mouse_state: &MouseState,
        out: &mut Vec<Vertex>,
    ) -> (TextItem, InteractionResult) {
        let mut click = ClickResult::None;
        let mut cursor = CursorIcon::Default;

        // build rectangle
        let title_bar_background = Rectangle {
            x: self.x,
            y: self.y - TITLEBAR_HEIGHT,
            width: self.width,
            height: TITLEBAR_HEIGHT,
        };
        title_bar_background.draw(screen_config, DARK_GRAY, TOP_RADIUS_6, out);

        // add button for closing the window
        let close_window_button = Rectangle::new(
            self.x + self.width - PAD_16 - PAD_8 - PAD_4,
            self.y - TITLEBAR_HEIGHT + PAD_8 + PAD_4,
            15.0,
            5.0,
        )
        .draw_style()
        .interactive(Some(mouse_state))
        .draw(screen_config, LIGHT_GRAY, NO_RADIUS, out);

        if close_window_button.hovered {
            cursor = CursorIcon::Pointer;
            if mouse_state.left_clicked {
                click = match self.window_kind {
                    WindowKind::Sequencer => ClickResult::ToggleSequencerWindow,
                    WindowKind::Playlist => ClickResult::TogglePlaylistWindow,
                    WindowKind::Mixer => ClickResult::ToggleMixerWindow,
                    WindowKind::PianoRoll => ClickResult::TogglePianoRollWindow,
                    WindowKind::TrackDetail(usize) => ClickResult::ToggleTrackWindow(usize),
                }
            }
        }
        // build text item
        let window_title = TextItem {
            text: title.to_string(),
            x: self.center_title_x(title),
            y: self.y - TITLEBAR_HEIGHT + PAD_4,
            color: WHITE,
            size: 18.0,
            font: ROBOTO,
        };
        (window_title, InteractionResult { click, cursor })
    }
}
