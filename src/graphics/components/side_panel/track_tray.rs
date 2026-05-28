use winit::window::CursorIcon;

use crate::project::Track;
use crate::{
    app::MouseState,
    graphics::{
        color::{BLACK, LIGHT_GRAY, LIGHT_GRAY_HOVER, PEBBLE, WHITE},
        components::side_panel::{PATTERN_TRAY_HEADER_MARGIN, PATTERN_TRAY_ITEM_GAP, TRAY_WIDTH},
        font::{truncate_text, TextItem, ROBOTO, TITLE},
        primitives::*,
        side_panel::{draw_title, PATTERN_TRAY_ITEM_HEIGHT},
        widgets::{Rectangle, TOOLBAR_Y},
        ClickResult,
    },
};

pub fn draw(mouse_state: &MouseState, screen_config: &ScreenConfig, tracks: &[Track]) -> (Vec<Vertex>, Vec<TextItem>, ClickResult, CursorIcon) {
    // setup
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut text_items: Vec<TextItem> = Vec::new();
    let mut cursor_icon: CursorIcon = CursorIcon::Default;
    let mut click_result: ClickResult = ClickResult::None;

    let track_tray = Rectangle {
        x: 0.0,
        y: TOOLBAR_Y,
        width: TRAY_WIDTH,
        height: screen_config.height as f32 - TOOLBAR_Y,
    };
    vertices.extend(track_tray.draw(screen_config, PEBBLE, NO_RADIUS));
    text_items.push(TextItem {
        text: "Tracks".to_string(),
        x: track_tray.x + PAD_8,
        y: TOOLBAR_Y + PAD_8,
        size: TITLE,
        color: WHITE,
        font: ROBOTO,
    });
    text_items.push(draw_title("Tracks", (track_tray.x, track_tray.y)));

    for (i, track) in tracks.into_iter().enumerate() {
        let button_x = track_tray.x + PAD_16;
        let button_y = PATTERN_TRAY_HEADER_MARGIN + (PATTERN_TRAY_ITEM_GAP * i as f32) + PAD_32;
        let track_button = Rectangle {
            x: button_x,
            y: button_y,
            width: track_tray.width - PAD_32,
            height: PATTERN_TRAY_ITEM_HEIGHT,
        };

        let track_button_color = if track_button.is_hovered(mouse_state.x, mouse_state.y) {
            LIGHT_GRAY_HOVER
        } else {
            LIGHT_GRAY
        };

        vertices.extend(track_button.draw(screen_config, track_button_color, RADIUS_4));
        if track_button.is_hovered(mouse_state.x, mouse_state.y) {
            cursor_icon = CursorIcon::Pointer;
            if mouse_state.left_double_clicked {
                click_result = ClickResult::ToggleTrackWindow(i);
            }
        }

        text_items.push(TextItem {
            text: truncate_text(&track.data.name, 9),
            x: track_button.x + PAD_4,
            y: PATTERN_TRAY_HEADER_MARGIN + (PATTERN_TRAY_ITEM_GAP * i as f32) + PAD_32 + PAD_2,
            size: 16.0,
            color: BLACK,
            font: ROBOTO,
        });
    }

    (vertices, text_items, click_result, cursor_icon)
}
