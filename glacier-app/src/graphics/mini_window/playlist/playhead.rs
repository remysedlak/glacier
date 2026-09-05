use crate::{
    app::ScrollOffset,
    graphics::{
        geometry::Rectangle,
        mini_window::{
            playlist::grid::{GRID_X_ORIGIN, PLAYHEAD_WIDTH, PLAYLIST_STEP_GAP},
            MiniWindow,
        },
        primitives::{PAD_16, PAD_64},
    },
};

pub fn draw(beat: f32, window: &MiniWindow, scroll_offset: &ScrollOffset) -> Rectangle {
    // @TODO: replace current_step with current_sample
    Rectangle::new(
        window.x + (beat * PLAYLIST_STEP_GAP) + PAD_16 + GRID_X_ORIGIN - scroll_offset.x,
        window.y + PAD_64,
        PLAYHEAD_WIDTH,
        window.height,
    )
}
