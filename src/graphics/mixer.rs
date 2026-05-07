use crate::color::*;
use crate::graphics::{ScreenConfig, Vertex};
use crate::ui::*;

pub fn draw(window: &MiniWindow, master_volume: &mut f32, screen_config: &ScreenConfig) -> (Vec<Vertex>, Vec<(String, f32, f32)>) {
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut text_items: Vec<(String, f32, f32)> = Vec::new();

    let window_background = Rectangle {
        x: window.x,
        y: window.y,
        width: window.width,
        height: window.height,
    };
    vertices.extend(window_background.draw(&screen_config, BACKGROUND));
    // titlebar rectangle
    let titlebar = Rectangle {
        x: window.x,
        y: window.y - TITLEBAR_HEIGHT,
        width: window.width,
        height: TITLEBAR_HEIGHT,
    };
    vertices.extend(titlebar.draw(&screen_config, DARK_GRAY));
    // titlebar text
    text_items.push((window.title.to_string(), window.x + window.width / 2.2, window.y - TITLEBAR_HEIGHT + 4.0));

    // master slider
    vertices.extend(draw_slider(master_volume, window.x, window.y, &screen_config));

    // text buffers
    let label = &format!("{:.2}", master_volume);
    text_items.push((label.to_string(), window.x, window.y));
    (vertices, text_items)
}
