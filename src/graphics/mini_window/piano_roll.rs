use winit::window::CursorIcon;

use crate::{
    app::MouseState,
    graphics::{
        font::TextItem,
        mini_window::{MiniWindow, MINI_WINDOW_BACKGROUND},
        primitives::{ScreenConfig, Vertex},
        widgets::{window_background, window_title_bar},
        ClickResult,
    },
};

pub fn draw(window: &MiniWindow, mouse_state: &MouseState, screen_config: &ScreenConfig) -> (Vec<Vertex>, Vec<TextItem>, ClickResult, CursorIcon) {
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut text_items: Vec<TextItem> = Vec::new();
    let mut cursor_icon = CursorIcon::Default;
    let mut click_result = ClickResult::None;

    let playlist_background = window_background(&window);
    vertices.extend(playlist_background.draw(&screen_config, MINI_WINDOW_BACKGROUND));

    // titlebar
    let (titlebar_verts, titlebar_texts, result, cursor) = window_title_bar(&window, screen_config, mouse_state);
    if !matches!(cursor, CursorIcon::Default) {
        cursor_icon = cursor;
    }
    click_result = click_result.or(result);
    vertices.extend(titlebar_verts);
    text_items.push(titlebar_texts);
    (vertices, text_items, click_result, cursor_icon)
}
