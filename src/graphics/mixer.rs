use crate::colors::*;
use crate::graphics::{make_text_buffer, ScreenConfig, Vertex};
use crate::ui::*;

pub fn draw(
    window: &MiniWindow,
    master_volume: &mut f32,
    font_system: &mut glyphon::FontSystem,
    screen_config: &ScreenConfig,
) -> (Vec<Vertex>, Vec<(glyphon::Buffer, f32, f32)>) {
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut text_items: Vec<(glyphon::Buffer, f32, f32)> = Vec::new();

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
    text_items.push((
        make_text_buffer(font_system, &window.title, 14.0, 22.0, None),
        window.x + window.width / 2.2,
        window.y - TITLEBAR_HEIGHT + 4.0,
    ));

    // master slider
    vertices.extend(draw_slider(master_volume, window.x, window.y, &screen_config));

    // text buffers
    let label = &format!("{:.2}", master_volume);
    text_items.push((make_text_buffer(font_system, label, 14.0, 22.0, Some((0, 0, 0))), window.x, window.y));
    (vertices, text_items)
}
