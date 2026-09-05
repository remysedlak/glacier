// ruler.rs
use super::grid::{GRID_X_ORIGIN, PLAYLIST_STEP_GAP};
use crate::app::ScrollOffset;
use crate::graphics::color::{BLACK, GREEN, ORANGE};
use crate::graphics::font::MONOSPACED;
use crate::graphics::geometry::{BorderStyle, Rectangle, ICON_BORDER};
use crate::graphics::primitives::{Vertex, NO_RADIUS, PAD_64};
use crate::graphics::{
    color::WHITE,
    font::{TextItem, ROBOTO},
    mini_window::MiniWindow,
    primitives::{ScreenConfig, PAD_16},
};

pub fn draw(
    window: &MiniWindow,
    screen_config: &ScreenConfig,
    step_count: usize,
    scroll_offset: &ScrollOffset,
) -> (Vec<Vertex>, Vec<TextItem>) {
    let mut text_items: Vec<TextItem> = vec![];
    let mut vertices: Vec<Vertex> = vec![];

    let background = Rectangle::new(
        window.x + PAD_16 + GRID_X_ORIGIN - 4.0,
        window.y + 44.0,
        window.width - PAD_16 - GRID_X_ORIGIN,
        20.0,
    )
    .draw_style()
    .bordered(Some(BorderStyle {
        size: 1.0,
        color: WHITE,
    }))
    .draw(screen_config, BLACK, NO_RADIUS, &mut vertices);

    for step in (0..step_count).step_by(16) {
        let group = step / 4;
        text_items.push(TextItem {
            text: format!("{group}"),
            x: window.x + (step as f32 * PLAYLIST_STEP_GAP) + PAD_16 + GRID_X_ORIGIN
                - scroll_offset.x,
            y: window.y + 42.0,
            size: 16.0,
            font: MONOSPACED,
            color: GREEN,
        });
    }

    (vertices, text_items)
}
