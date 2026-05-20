use crate::app::MouseState;
use crate::graphics::color::{BLACK, WHITE};
use crate::graphics::primitives::{ScreenConfig, PAD_8};
use crate::graphics::widgets::TextItem;
use crate::graphics::{ClickResult, CursorIcon, PatternData, Rectangle, Vertex, PAD_16, PAD_4, TOOLBAR_THICKNESS, TOOLBAR_Y};

pub fn draw(
    screen_config: &ScreenConfig,
    patterns: &[PatternData],
    active_pattern_id: usize,
    mouse_state: &MouseState,
) -> (Vec<Vertex>, Vec<TextItem>, ClickResult, CursorIcon) {
    // setup return
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut texts: Vec<TextItem> = Vec::new();
    let mut click_result = ClickResult::None;
    let mut cursor_icon = CursorIcon::Default;

    // patterns tray
    let pattern_tray = Rectangle {
        x: screen_config.width as f32 - 128.0,
        y: TOOLBAR_Y,
        width: 128.0,
        height: screen_config.height as f32 - TOOLBAR_THICKNESS,
    };
    vertices.extend(pattern_tray.draw(&screen_config, crate::graphics::color::PASCAL));
    if pattern_tray.is_hovered_left_edge(mouse_state.x, mouse_state.y) {
        cursor_icon = CursorIcon::ColResize
    }

    for (i, pattern) in patterns.iter().enumerate() {
        texts.push(TextItem {
            text: pattern.name.to_string(),
            x: screen_config.width as f32 - 96.0,
            y: 52.0 + (32.0 * i as f32) + 24.0,
            size: 16.0,
            color: BLACK,
        });
    }

    // add pattern button
    let add_pattern_button = Rectangle {
        x: screen_config.width as f32 - 32.0,
        y: TOOLBAR_Y + 12.0,
        width: 16.0,
        height: 16.0,
    };
    vertices.extend(add_pattern_button.draw(&screen_config, add_pattern_button.hover_color(mouse_state.x, mouse_state.y)));
    if add_pattern_button.is_hovered(mouse_state.x, mouse_state.y) {
        cursor_icon = CursorIcon::Pointer;
        if mouse_state.left_clicked {
            click_result = ClickResult::AddPlaylist;
        }
    }

    for (i, pattern) in patterns.iter().enumerate() {
        let button_x = screen_config.width as f32 - 128.0 + PAD_16;
        let button_y = 52.0 + (32.0 * i as f32) + 24.0;

        let pattern_button = Rectangle {
            x: button_x,
            y: button_y,
            width: 96.0,
            height: 24.0,
        };
        if i == active_pattern_id {
            let indicator = Rectangle {
                x: screen_config.width as f32 - 128.0 + PAD_8,
                y: 52.0 + (32.0 * i as f32) + 24.0,
                width: 4.0,
                height: 24.0,
            };
            vertices.extend(indicator.draw(&screen_config, crate::graphics::color::ORANGE));
        }
        vertices.extend(pattern_button.draw(&screen_config, pattern_button.hover_color(mouse_state.x, mouse_state.y)));
        if pattern_button.is_hovered(mouse_state.x, mouse_state.y) {
            cursor_icon = CursorIcon::Pointer;
            if mouse_state.left_clicked {
                click_result = ClickResult::SelectPattern(pattern.id as usize);
            }
            if mouse_state.right_clicked {
                click_result = ClickResult::OpenPatternMenu(button_x, button_y, pattern.id as usize);
            }
        }
    }
    texts.push(TextItem {
        text: "Patterns".to_string(),
        x: screen_config.width as f32 - 128.0 + PAD_8,
        y: TOOLBAR_Y + PAD_8,
        size: 18.0,
        color: WHITE,
    });

    (vertices, texts, click_result, cursor_icon)
}
