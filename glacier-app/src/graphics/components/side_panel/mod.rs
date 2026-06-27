use crate::graphics::{
    color::WHITE,
    font::{TextItem, ROBOTO, TITLE},
    primitives::PAD_8,
};

pub mod pattern_tray;
pub mod track_tray;

pub const PATTERN_TRAY_ITEM_WIDTH: f32 = 96.0;
pub const PATTERN_TRAY_ITEM_HEIGHT: f32 = 20.0;
pub const DEFAULT_TRAY_WIDTH: f32 = 128.0;
pub const PATTERN_TRAY_HEADER_MARGIN: f32 = 64.0;
pub const PATTERN_TRAY_ITEM_GAP: f32 = 32.0;

pub fn draw_title(title: &str, origin: (f32, f32)) -> TextItem {
    TextItem {
        text: title.to_string(),
        x: origin.0 + PAD_8,
        y: origin.1 + PAD_8,
        size: TITLE,
        color: WHITE,
        font: ROBOTO,
    }
}
