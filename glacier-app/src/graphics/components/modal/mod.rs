use crate::graphics::{
    color::DARK_GRAY,
    font::TextItem,
    primitives::{ScreenConfig, Vertex, RADIUS_8},
    widgets::Rectangle,
};

const MODAL_HEIGHT: f32 = 256.0;
const MODAL_WIDTH: f32 = 512.0;

pub fn draw(screen_config: &ScreenConfig) -> (Vec<Vertex>, Vec<TextItem>) {
    let mut vertices: Vec<Vertex> = Vec::new();
    let text_items: Vec<TextItem> = Vec::new();

    let modal_background = Rectangle {
        x: (screen_config.width as f32 / 2.0) - MODAL_WIDTH / 2.0,
        y: (screen_config.height as f32 / 2.0) - MODAL_HEIGHT / 2.0,
        height: MODAL_HEIGHT,
        width: MODAL_WIDTH,
    };
    modal_background.draw(screen_config, DARK_GRAY, RADIUS_8, &mut vertices);
    // text_items.push(draw_title(title, ())));
    (vertices, text_items)
}
