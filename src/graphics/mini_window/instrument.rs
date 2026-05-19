use crate::app::MouseState;
use crate::graphics::primitives::{PAD_16, PAD_64};
use crate::graphics::{
    color::{BACKGROUND, DARK_GRAY},
    mini_window::MiniWindow,
    primitives::{ScreenConfig, Vertex},
    widgets::{window_background, window_title_bar},
    {ClickResult, Rectangle, TextItem},
};
use crate::project::Instrument;
use winit::window::CursorIcon;

pub fn draw(
    window: &MiniWindow,
    mouse_state: &MouseState,
    screen_config: &ScreenConfig,
    track: &Instrument,
) -> (Vec<Vertex>, Vec<TextItem>, ClickResult, CursorIcon) {
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut text_items: Vec<TextItem> = Vec::new();
    let mut click_result = ClickResult::None;
    let mut cursor_icon = CursorIcon::Default;

    // window background
    let window_background = window_background(&window);
    vertices.extend(window_background.draw(&screen_config, BACKGROUND));

    // titlebar
    let (titlebar_verts, titlebar_texts) = window_title_bar(&window);
    vertices.extend(titlebar_verts.draw(&screen_config, DARK_GRAY));
    text_items.push(titlebar_texts);

    let rectangle = Rectangle {
        x: window.x + PAD_64,
        y: window.y + PAD_64,
        width: 32.0,
        height: 32.0,
    };
    text_items.push(TextItem {
        text: track.data.path.clone(),
        x: window.x + PAD_16,
        y: window.y + PAD_16,
        size: 18.0,
    });
    vertices.extend(rectangle.draw(&screen_config, DARK_GRAY));
    (vertices, text_items, click_result, cursor_icon)
}
