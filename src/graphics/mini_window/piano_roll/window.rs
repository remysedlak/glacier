use crate::{
    app::MouseState,
    graphics::{
        color::{BLACK, BLUE, BLUE_HOVER, DARK_BLUE, DARK_BLUE_HOVER, LL_GRAY},
        font::TextItem,
        mini_window::{
            piano_roll::{
                black_piano_step_hover_color, white_piano_step_hover_color, BLACK_SEMITONE_INDEXES, OCTAVE_GAP, PIANO_ROLL_MARGIN, PIANO_ROLL_WIDTH,
                SEMITONE_GAP, SEMITONE_HEIGHT,
            },
            MINI_WINDOW_BACKGROUND,
        },
        primitives::{ScreenConfig, Vertex, PAD_16, PAD_2, PAD_32, PAD_4, PAD_8},
        widgets::{window_background, window_title_bar, Rectangle, TITLEBAR_HEIGHT},
        ClickResult, MiniWindow,
    },
};
use winit::window::CursorIcon;

pub fn draw(
    window: &MiniWindow,
    mouse_state: &MouseState,
    scroll_x: f32,
    scroll_y: f32,
    screen_config: &ScreenConfig,
) -> (
    Vec<Vertex>,
    Vec<TextItem>,
    Vec<Vertex>,
    Vec<TextItem>,
    Vec<Vertex>,
    Vec<TextItem>,
    ClickResult,
    CursorIcon,
) {
    let mut static_vertices: Vec<Vertex> = Vec::new();
    let mut static_text_items: Vec<TextItem> = Vec::new();
    let mut piano_key_vertices: Vec<Vertex> = Vec::new();
    let mut piano_key_text_items: Vec<TextItem> = Vec::new();
    let mut grid_vertices: Vec<Vertex> = Vec::new();
    let mut grid_text_items: Vec<TextItem> = Vec::new();
    let mut cursor_icon = CursorIcon::Default;
    let mut click_result = ClickResult::None;

    let playlist_background = window_background(&window);
    static_vertices.extend(playlist_background.draw(&screen_config, MINI_WINDOW_BACKGROUND));

    // titlebar
    let (titlebar_verts, titlebar_texts, result, cursor) = window_title_bar(&window, screen_config, mouse_state);
    if !matches!(cursor, CursorIcon::Default) {
        cursor_icon = cursor;
    }
    click_result = click_result.or(result);
    static_vertices.extend(titlebar_verts);
    static_text_items.push(titlebar_texts);

    // toolbar
    let bottom_toolbar_background = Rectangle {
        x: window.x + PAD_8 + PIANO_ROLL_WIDTH,
        y: window.y + TITLEBAR_HEIGHT + PAD_4,
        width: window.width - PAD_16 - PAD_8 - PIANO_ROLL_WIDTH,
        height: 30.0,
    };
    static_vertices.extend(bottom_toolbar_background.draw(&screen_config, LL_GRAY));

    // piano
    for octave in 0..9 {
        for semitone in 0..12 {
            if BLACK_SEMITONE_INDEXES.contains(&semitone) {
                // two pieces for black keys, one for the black key itself and one for the false white key in front of it
                let white_key_width = 24.0;
                let black_key_width = PIANO_ROLL_WIDTH - white_key_width;
                let black_piano_key = Rectangle {
                    x: window.x + PAD_8,
                    y: window.y + (semitone as f32 * SEMITONE_GAP) + (octave as f32 * OCTAVE_GAP) + PIANO_ROLL_MARGIN + PAD_8 - scroll_y,
                    height: SEMITONE_HEIGHT,
                    width: black_key_width,
                };
                let white_piano_key = Rectangle {
                    x: window.x + black_key_width + PAD_8,
                    y: window.y + (semitone as f32 * SEMITONE_GAP) + (octave as f32 * OCTAVE_GAP) + PIANO_ROLL_MARGIN + PAD_8 - scroll_y,
                    height: SEMITONE_HEIGHT,
                    width: white_key_width,
                };
                piano_key_vertices.extend(black_piano_key.draw(
                    screen_config,
                    black_piano_step_hover_color(&black_piano_key, mouse_state.x, mouse_state.y),
                ));
                piano_key_vertices.extend(white_piano_key.draw(
                    screen_config,
                    white_piano_step_hover_color(&white_piano_key, mouse_state.x, mouse_state.y, semitone),
                ));
            } else {
                // full white piano key
                let piano_key = Rectangle {
                    x: window.x + PAD_8,
                    y: window.y + (semitone as f32 * SEMITONE_GAP) + (octave as f32 * OCTAVE_GAP) + PIANO_ROLL_MARGIN + PAD_8 - scroll_y,
                    height: SEMITONE_HEIGHT,
                    width: PIANO_ROLL_WIDTH,
                };
                piano_key_vertices.extend(piano_key.draw(
                    screen_config,
                    white_piano_step_hover_color(&piano_key, mouse_state.x, mouse_state.y, semitone),
                ));
            }
            // draw steps to draw the melody on
            for step_index in 0..127 {
                let piano_roll_step = Rectangle {
                    x: window.x + (step_index as f32 * PAD_16) + PIANO_ROLL_MARGIN + PAD_8 - scroll_x,
                    y: window.y + (semitone as f32 * SEMITONE_GAP) + (octave as f32 * OCTAVE_GAP) + PIANO_ROLL_MARGIN + PAD_8 - scroll_y,
                    height: SEMITONE_HEIGHT,
                    width: 15.0,
                };
                let hovered: bool = piano_roll_step.is_hovered(mouse_state.x, mouse_state.y) && !mouse_state.left_click_held;
                let color = if hovered {
                    if (step_index / 4) % 2 == 0 {
                        BLUE_HOVER
                    } else {
                        DARK_BLUE_HOVER
                    }
                } else {
                    if (step_index / 4) % 2 == 0 {
                        BLUE
                    } else {
                        DARK_BLUE
                    }
                };
                grid_vertices.extend(piano_roll_step.draw(screen_config, color));
            }
        }

        // push label for C note
        let c_label = format!("C{}", 8 - octave);
        piano_key_text_items.push(TextItem {
            text: c_label.to_string(),
            x: window.x + PAD_32 + PAD_16 + PAD_2 + PAD_8,
            y: window.y + (11.0 * SEMITONE_GAP) + (octave as f32 * OCTAVE_GAP) + PIANO_ROLL_MARGIN + PAD_8 - scroll_y,
            size: 10.0,
            font: "mono",
            color: BLACK,
        });
    }
    (
        static_vertices,
        static_text_items,
        piano_key_vertices,
        piano_key_text_items,
        grid_vertices,
        grid_text_items,
        click_result,
        cursor_icon,
    )
}
