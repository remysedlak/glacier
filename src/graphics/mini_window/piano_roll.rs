use winit::window::CursorIcon;

use crate::{
    app::MouseState,
    graphics::{
        color::{BLACK, DARK_GRAY, EGG_WHITE, LL_GRAY, PEBBLE, WHITE},
        font::TextItem,
        mini_window::{MiniWindow, MINI_WINDOW_BACKGROUND},
        primitives::{ScreenConfig, Vertex, PAD_16, PAD_2, PAD_32, PAD_4, PAD_64, PAD_8},
        widgets::{window_background, window_title_bar, Rectangle},
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

    // piano
    let black_semitones = [1, 3, 5, 8, 10];
    let mut white_keys: Vec<Vertex> = Vec::new();
    let mut black_keys: Vec<Vertex> = Vec::new();
    let mut black_note_hovered = false;
    for octave in 0..5 {
        for semitone in 0..12 {
            if black_semitones.contains(&semitone) {
                let gap = if semitone > 5 { 34.0 } else { 12.0 };
                let piano_key = Rectangle {
                    x: window.x + PAD_8,
                    y: window.y + PAD_8 + black_keys.len() as f32 * 3.5 + gap + octave as f32 * 42.0,
                    height: 16.0,
                    width: 40.0,
                };
                black_keys.extend(piano_key.draw(screen_config, piano_key.black_piano_step_hover_color(mouse_state.x, mouse_state.y)));
                if piano_key.is_hovered(mouse_state.x, mouse_state.y) {
                    black_note_hovered = true;
                }
            } else {
                let piano_key = Rectangle {
                    x: window.x + PAD_8,
                    y: window.y + PAD_8 + (white_keys.len() as f32 * 3.5),
                    height: 20.0,
                    width: 64.0,
                };
                white_keys.extend(piano_key.draw(
                    screen_config,
                    piano_key.white_piano_step_hover_color(mouse_state.x, mouse_state.y, semitone, black_note_hovered),
                ));
            }
        }

        // push label for C note
        let c_label = format!("C{}", octave);
        text_items.push(TextItem {
            text: c_label.to_string(),
            x: window.x + PAD_32 + PAD_16 + PAD_8,
            y: window.y + PAD_8 + (white_keys.len() as f32 * 3.5) - PAD_16 - PAD_4,
            size: 10.0,
            font: "mono",
            color: PEBBLE,
        });
    }
    vertices.extend(white_keys);
    vertices.extend(black_keys);

    (vertices, text_items, click_result, cursor_icon)
}
