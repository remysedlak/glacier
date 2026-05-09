use crate::color::*;
use crate::graphics::{
    ui::*,
    widgets::{draw_slider, window_background, window_title_bar},
    ScreenConfig, TextItem, Vertex,
};

pub fn draw(window: &MiniWindow, master_volume: f32, screen_config: &ScreenConfig) -> (Vec<Vertex>, Vec<TextItem>) {
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut text_items: Vec<TextItem> = Vec::new();

    // window background
    let window_background = window_background(&window);
    vertices.extend(window_background.draw(&screen_config, BACKGROUND));

    // window titlebar
    let (titlebar_verts, titlebar_texts) = window_title_bar(&window);
    vertices.extend(titlebar_verts.draw(&screen_config, DARK_GRAY));
    text_items.push(titlebar_texts);

    // master slider
    vertices.extend(draw_slider(master_volume, window.x, window.y, &screen_config));

    // text buffers
    let label = &format!("{:.2}", master_volume);
    text_items.push(TextItem {
        text: label.to_string(),
        x: window.x,
        y: window.y,
    });
    (vertices, text_items)
}
