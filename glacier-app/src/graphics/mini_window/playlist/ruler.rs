// ruler.rs
use super::grid::{GRID_X_ORIGIN, PLAYLIST_STEP_GAP};
use crate::app::ScrollOffset;
use crate::graphics::primitives::Vertex;
use crate::graphics::{
    color::WHITE,
    font::{TextItem, ROBOTO},
    mini_window::MiniWindow,
    primitives::{ScreenConfig, PAD_16, PAD_8},
};

pub fn draw(
    window: &MiniWindow,
    screen_config: &ScreenConfig,
    step_count: usize,
    scroll_offset: &ScrollOffset,
) -> (Vec<Vertex>, Vec<TextItem>) {
    let mut text_items: Vec<TextItem> = vec![];
    let mut vertices: Vec<Vertex> = vec![];

    for step in (0..step_count).step_by(16) {
        let group = step / 4;
        text_items.push(TextItem {
            text: format!("{group}"),
            x: window.x + (step as f32 * PLAYLIST_STEP_GAP) + PAD_16 + GRID_X_ORIGIN
                - scroll_offset.x,
            y: window.y + 44.0,
            size: 18.0,
            font: ROBOTO,
            color: WHITE,
        });
    }

    (vertices, text_items)
}
