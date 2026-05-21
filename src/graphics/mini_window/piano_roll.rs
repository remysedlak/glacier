use crate::{
    app::MouseState,
    graphics::{
        color::{BLACK, BLUE, DARK_BLUE, DARK_GRAY, EGG_WHITE, EGG_WHITE_HOVER, PEBBLE, WHITE},
        font::TextItem,
        mini_window::{MiniWindow, MINI_WINDOW_BACKGROUND},
        primitives::{ScreenConfig, Vertex, PAD_16, PAD_2, PAD_32, PAD_64, PAD_8},
        widgets::{window_background, window_title_bar, Rectangle},
        ClickResult,
    },
};
use winit::window::CursorIcon;

// return color for hover logic
pub fn black_piano_step_hover_color(rect: &Rectangle, mx: f32, my: f32) -> (f32, f32, f32) {
    if rect.is_hovered(mx, my) {
        DARK_GRAY
    } else {
        BLACK
    }
}
pub fn white_piano_step_hover_color(rect: &Rectangle, mx: f32, my: f32, index: u32) -> (f32, f32, f32) {
    if rect.is_hovered(mx, my) && index == 11 {
        EGG_WHITE_HOVER
    } else if index == 11 {
        EGG_WHITE
    } else if rect.is_hovered(mx, my) {
        EGG_WHITE_HOVER
    } else {
        WHITE
    }
}

pub fn draw(
    window: &MiniWindow,
    mouse_state: &MouseState,
    scroll_x: f32,
    scroll_y: f32,
    screen_config: &ScreenConfig,
) -> (Vec<Vertex>, Vec<TextItem>, ClickResult, CursorIcon) {
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

    pub const SEMITONE_GAP: f32 = 18.0;
    pub const OCTAVE_GAP: f32 = 216.0;
    pub const PIANO_ROLL_MARGIN: f32 = 64.0;

    for octave in 0..9 {
        for semitone in 0..12 {
            if black_semitones.contains(&semitone) {
                let black_piano_key = Rectangle {
                    x: window.x + PAD_8 - scroll_x,
                    y: window.y + PAD_8 + semitone as f32 * SEMITONE_GAP + (octave as f32 * OCTAVE_GAP) - scroll_y + PIANO_ROLL_MARGIN,
                    height: 16.0,
                    width: 40.0,
                };
                let white_piano_key = Rectangle {
                    x: window.x + PAD_8 - scroll_x + 40.0,
                    y: window.y + PAD_8 + semitone as f32 * SEMITONE_GAP + (octave as f32 * OCTAVE_GAP) - scroll_y + PIANO_ROLL_MARGIN,
                    height: 16.0,
                    width: 24.0,
                };
                vertices.extend(black_piano_key.draw(
                    screen_config,
                    black_piano_step_hover_color(&black_piano_key, mouse_state.x, mouse_state.y),
                ));
                vertices.extend(white_piano_key.draw(
                    screen_config,
                    white_piano_step_hover_color(&white_piano_key, mouse_state.x, mouse_state.y, semitone),
                ));
            } else {
                let piano_key = Rectangle {
                    x: window.x + PAD_8 - scroll_x,
                    y: window.y + PAD_8 + semitone as f32 * SEMITONE_GAP + (octave as f32 * OCTAVE_GAP) - scroll_y + PIANO_ROLL_MARGIN,
                    height: 16.0,
                    width: 64.0,
                };
                vertices.extend(piano_key.draw(
                    screen_config,
                    white_piano_step_hover_color(&piano_key, mouse_state.x, mouse_state.y, semitone),
                ));
            }
            for i in 0..15 {
                let shaded = (i / 4) % 2 == 0;
                let color = if shaded { DARK_BLUE } else { BLUE };
                let piano_roll_step = Rectangle {
                    x: window.x + PAD_8 - scroll_x + PAD_64 + i as f32 * PAD_16,
                    y: window.y + PAD_8 + semitone as f32 * SEMITONE_GAP + (octave as f32 * OCTAVE_GAP) - scroll_y + PIANO_ROLL_MARGIN,
                    height: 16.0,
                    width: 15.0,
                };
                vertices.extend(piano_roll_step.draw(screen_config, color));
            }
        }

        // push label for C note
        let c_label = format!("C{}", octave);
        text_items.push(TextItem {
            text: c_label.to_string(),
            x: window.x + PAD_8 - scroll_x + PAD_32 + PAD_16 + PAD_2,
            y: window.y + PAD_8 + 11.0 * SEMITONE_GAP + (octave as f32 * OCTAVE_GAP) - scroll_y + PIANO_ROLL_MARGIN,
            size: 10.0,
            font: "mono",
            color: PEBBLE,
        });
    }
    (vertices, text_items, click_result, cursor_icon)
}
