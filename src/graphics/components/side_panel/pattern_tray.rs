use crate::app::MouseState;
use crate::graphics::color::LIGHT_GRAY_HOVER;
use crate::graphics::components::side_panel::{
    draw_title, PATTERN_TRAY_HEADER_MARGIN, PATTERN_TRAY_ITEM_GAP, PATTERN_TRAY_ITEM_HEIGHT, PATTERN_TRAY_ITEM_WIDTH, TRAY_WIDTH,
};
use crate::graphics::font::TITLE;
use crate::graphics::{
    color::{BLACK, LIGHT_GRAY, PEBBLE, WHITE},
    font::{TextItem, ROBOTO},
    primitives::{ScreenConfig, PAD_16, PAD_32, PAD_8},
    {ClickResult, CursorIcon, PatternData, Rectangle, Vertex, NO_RADIUS, PAD_2, TOOLBAR_THICKNESS, TOOLBAR_Y},
};

pub fn draw(
    screen_config: &ScreenConfig,
    patterns: &[PatternData],
    active_pattern_id: usize,
    mouse_state: &MouseState,
) -> (Vec<Vertex>, Vec<TextItem>, ClickResult, CursorIcon) {
    // setup return
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut text_items: Vec<TextItem> = Vec::new();
    let mut click_result = ClickResult::None;
    let mut cursor_icon = CursorIcon::Default;

    // patterns tray
    let pattern_tray = Rectangle {
        x: screen_config.width as f32 - 128.0,
        y: TOOLBAR_Y,
        width: TRAY_WIDTH,
        height: screen_config.height as f32 - TOOLBAR_THICKNESS,
    };
    vertices.extend(pattern_tray.draw(&screen_config, PEBBLE, NO_RADIUS));
    if pattern_tray.is_hovered_left_edge(mouse_state.x, mouse_state.y) {
        cursor_icon = CursorIcon::ColResize
    }
    // title
    text_items.push(draw_title("Patterns", (pattern_tray.x, pattern_tray.y)));

    // add pattern button
    let add_pattern_button = Rectangle {
        x: screen_config.width as f32 - PAD_32,
        y: pattern_tray.y + PAD_8,
        width: 16.0,
        height: 16.0,
    };
    let add_pattern_button_color = if add_pattern_button.is_hovered(mouse_state.x, mouse_state.y) {
        LIGHT_GRAY_HOVER
    } else {
        LIGHT_GRAY
    };
    vertices.extend(add_pattern_button.draw(&screen_config, add_pattern_button_color, NO_RADIUS));
    if add_pattern_button.is_hovered(mouse_state.x, mouse_state.y) {
        cursor_icon = CursorIcon::Pointer;
        if mouse_state.left_clicked {
            click_result = ClickResult::CreatePattern;
        }
    }

    for (i, pattern) in patterns.iter().enumerate() {
        let button_x = pattern_tray.x + PAD_16;
        let button_y = PATTERN_TRAY_HEADER_MARGIN + (PATTERN_TRAY_ITEM_GAP * i as f32) + PAD_32;

        let pattern_button = Rectangle {
            x: button_x,
            y: button_y,
            width: pattern_tray.width - PAD_32,
            height: PATTERN_TRAY_ITEM_HEIGHT,
        };
        if i == active_pattern_id {
            let indicator = Rectangle {
                x: pattern_tray.x + PAD_8,
                y: PATTERN_TRAY_HEADER_MARGIN + (PATTERN_TRAY_ITEM_GAP * i as f32) + PAD_32,
                width: 4.0,
                height: PATTERN_TRAY_ITEM_HEIGHT,
            };
            vertices.extend(indicator.draw(&screen_config, crate::graphics::color::ORANGE, [7.0, 7.0, 7.0, 7.0]));
        }

        let pattern_button_color = if pattern_button.is_hovered(mouse_state.x, mouse_state.y) {
            LIGHT_GRAY_HOVER
        } else {
            LIGHT_GRAY
        };

        vertices.extend(pattern_button.draw(&screen_config, pattern_button_color, [4.0, 4.0, 4.0, 4.0]));
        if pattern_button.is_hovered(mouse_state.x, mouse_state.y) {
            cursor_icon = CursorIcon::Pointer;
            if mouse_state.left_clicked {
                click_result = ClickResult::SelectPattern(pattern.id as usize);
            }
            if mouse_state.left_double_clicked {
                click_result = ClickResult::ToggleSequencerWindow;
            }
            if mouse_state.right_clicked {
                click_result = ClickResult::OpenPatternMenu(button_x, button_y, pattern.id as usize);
            }
        }
    }
    // load each pattern's name
    for (i, pattern) in patterns.iter().enumerate() {
        text_items.push(TextItem {
            text: pattern.name.to_string(),
            x: screen_config.width as f32 - PATTERN_TRAY_ITEM_WIDTH,
            y: PATTERN_TRAY_HEADER_MARGIN + (PATTERN_TRAY_ITEM_GAP * i as f32) + PAD_32 + PAD_2,
            size: 16.0,
            color: BLACK,
            font: ROBOTO,
        });
    }

    (vertices, text_items, click_result, cursor_icon)
}
