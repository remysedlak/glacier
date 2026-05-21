use winit::window::CursorIcon;

use crate::{
    app::MouseState,
    graphics::{
        color::*,
        mini_window::MiniWindow,
        primitives::ScreenConfig,
        widgets::{draw_slider, window_background, window_title_bar},
        ClickResult, TextItem, Vertex,
    },
};

pub fn draw(
    window: &MiniWindow,
    master_volume: f32,
    screen_config: &ScreenConfig,
    mouse_state: &MouseState,
) -> (Vec<Vertex>, Vec<TextItem>, ClickResult, CursorIcon) {
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut text_items: Vec<TextItem> = Vec::new();
    let mut click_result = ClickResult::None;
    let mut cursor_icon = CursorIcon::Default;

    // window background
    let window_background = window_background(&window);
    vertices.extend(window_background.draw(&screen_config, PURPLE));

    // window titlebar
    let (titlebar_verts, titlebar_texts, result, cursor) = window_title_bar(&window, screen_config, mouse_state);
    if !matches!(cursor, CursorIcon::Default) {
        cursor_icon = cursor;
    }
    click_result = click_result.or(result);
    vertices.extend(titlebar_verts);
    text_items.push(titlebar_texts);

    // master slider
    vertices.extend(draw_slider(master_volume, window.x, window.y, &screen_config));

    // text buffers
    let label = &format!("{:.2}", master_volume);
    text_items.push(TextItem {
        text: label.to_string(),
        x: window.x,
        size: 18.0,
        y: window.y,
        font: "roboto",
        color: WHITE,
    });
    (vertices, text_items, click_result, cursor_icon)
}
