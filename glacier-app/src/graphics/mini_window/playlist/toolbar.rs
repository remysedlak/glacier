use crate::graphics::{
    color::DARK_GRAY,
    font::TextItem,
    geometry::{Rectangle, ICON_BORDER},
    mini_window::MiniWindow,
    primitives::{ScreenConfig, Vertex, NO_RADIUS, PAD_16, PAD_32, PAD_4, PAD_8},
};

pub fn draw(window: &MiniWindow, screen_config: &ScreenConfig) -> (Vec<Vertex>, Vec<TextItem>) {
    let mut verts = vec![];
    let texts = vec![];
    let background = Rectangle::new(
        window.x + PAD_16,
        window.y + PAD_4,
        window.width - PAD_32 * 2.0,
        PAD_16 * 2.0,
    )
    .draw_style()
    .bordered(Some(ICON_BORDER))
    .draw(screen_config, DARK_GRAY, NO_RADIUS, &mut verts);

    for button in 0..5 {
        let _ = Rectangle::new(
            background.x + PAD_4 + 48.0 * button as f32,
            background.y + PAD_4,
            24.0,
            24.0,
        )
        .draw_style()
        .bordered(Some(ICON_BORDER))
        .draw(screen_config, DARK_GRAY, NO_RADIUS, &mut verts);
    }
    return (verts, texts);
}
