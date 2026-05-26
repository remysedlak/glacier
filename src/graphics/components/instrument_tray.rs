use crate::{
    app::MouseState,
    graphics::{
        color::{PEBBLE, WHITE},
        font::{TextItem, ROBOTO_FONT},
        primitives::{ScreenConfig, Vertex, NO_RADIUS, PAD_8},
        widgets::{Rectangle, TOOLBAR_Y},
    },
};

pub fn draw(mouse_state: &MouseState, screen_config: &ScreenConfig) -> (Vec<Vertex>, Vec<TextItem>) {
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut text_items: Vec<TextItem> = Vec::new();
    let background = Rectangle {
        x: 0.0,
        y: TOOLBAR_Y,
        width: 128.0,
        height: screen_config.height as f32 - TOOLBAR_Y,
    };

    vertices.extend(background.draw(screen_config, PEBBLE, NO_RADIUS));
    text_items.push(TextItem {
        text: "Audio Files".to_string(),
        x: PAD_8,
        y: TOOLBAR_Y + PAD_8,
        size: 16.0,
        color: WHITE,
        font: ROBOTO_FONT,
    });
    (vertices, text_items)
}
