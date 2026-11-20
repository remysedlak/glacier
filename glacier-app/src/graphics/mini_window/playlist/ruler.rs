// ruler.rs
use super::grid::{GRID_X_ORIGIN, PLAYLIST_STEP_GAP};
use crate::app::ScrollOffset;
use crate::graphics::font::Font::Mono;
use crate::graphics::geometry::{BorderStyle, Rectangle};
use crate::graphics::primitives::NO_RADIUS;
use crate::graphics::{
    FrameDrawContext,
    font::TextItem,
    mini_window::MiniWindow,
    primitives::{PAD_16, ScreenConfig},
};

pub fn draw(
    window: &MiniWindow,
    screen_config: &ScreenConfig,
    step_count: usize,
    scroll_offset: &ScrollOffset,
    frame: &mut FrameDrawContext<'_>,
) {
    let _background = Rectangle::new(
        window.x + PAD_16 + GRID_X_ORIGIN - 4.0,
        window.y + 44.0,
        window.width - PAD_16 - GRID_X_ORIGIN,
        20.0,
    )
    .draw_style()
    .bordered(Some(BorderStyle {
        size: 1.0,
        color: screen_config.palette.neutral.shade(7),
    }))
    .draw(
        screen_config,
        screen_config.palette.neutral.shade(0),
        NO_RADIUS,
        frame,
    );

    for step in (0..step_count).step_by(16) {
        let group = step / 4;
        frame.draw_texts(&[TextItem {
            text: format!("{group}"),
            x: window.x + (step as f32 * PLAYLIST_STEP_GAP) + PAD_16 + GRID_X_ORIGIN
                - scroll_offset.x,
            y: window.y + 42.0,
            size: 16.0,
            font: Mono,
            color: screen_config.palette.secondary.shade(4),
        }]);
    }
}
