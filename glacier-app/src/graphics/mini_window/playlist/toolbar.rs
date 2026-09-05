use crate::graphics::{
    color::{DARK_GRAY, WHITE},
    font::TextItem,
    geometry::{Rectangle, ICON_BORDER},
    mini_window::MiniWindow,
    primitives::{ScreenConfig, Vertex, NO_RADIUS, PAD_16, PAD_32},
};

pub fn draw(window: &MiniWindow, screen_config: &ScreenConfig) -> (Vec<Vertex>, Vec<TextItem>) {
    let mut verts = vec![];
    let mut texts = vec![];
    let _ = Rectangle::new(
        window.x + PAD_16,
        window.y + PAD_16,
        window.width - PAD_32 * 2.0,
        PAD_16 * 2.0,
    )
    .draw_style()
    .bordered(Some(ICON_BORDER))
    .draw(screen_config, DARK_GRAY, NO_RADIUS, &mut verts);
    return (verts, texts);
}
