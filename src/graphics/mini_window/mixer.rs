use winit::window::CursorIcon;

use crate::{
    app::MouseState,
    graphics::{
        color::*,
        font::ROBOTO_FONT,
        mini_window::MiniWindow,
        primitives::{ScreenConfig, BOTTOM_RADIUS_16, PAD_16, PAD_4, PAD_8},
        widgets::{draw_slider, window_background, window_title_bar, MIXER_TRACK_HEIGHT},
        ClickResult, TextItem, Vertex,
    },
};

pub const SLIDER_OFFSET: f32 = PAD_16;

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
    vertices.extend(window_background.draw(&screen_config, PURPLE, BOTTOM_RADIUS_16));

    // window titlebar
    let (titlebar_verts, titlebar_texts, result, cursor) = window_title_bar(&window, "Mixer", screen_config, mouse_state);
    if !matches!(cursor, CursorIcon::Default) {
        cursor_icon = cursor;
    }
    click_result = click_result.or(result);
    vertices.extend(titlebar_verts);
    text_items.push(titlebar_texts);

    // master slider
    let slider_x = window.x + SLIDER_OFFSET;
    let slider_y = window.y + SLIDER_OFFSET;

    vertices.extend(draw_slider(master_volume, slider_x, slider_y, screen_config));

    text_items.push(TextItem {
        text: format!("{:.2}", master_volume),
        x: slider_x,
        y: slider_y + MIXER_TRACK_HEIGHT + PAD_4,
        size: 18.0,
        font: ROBOTO_FONT,
        color: WHITE,
    });
    (vertices, text_items, click_result, cursor_icon)
}
