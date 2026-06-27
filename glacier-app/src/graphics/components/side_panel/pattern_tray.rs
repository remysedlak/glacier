use crate::app::MouseState;
use crate::graphics::color::LL_GRAY;
use crate::graphics::{
    color::{BLACK, LIGHT_GRAY, LIGHT_GRAY_HOVER, PEBBLE},
    components::{side_panel::*, toolbar::icon_color},
    font::{TextItem, ROBOTO},
    icons::{IconDraw, Tooltip},
    primitives::{ScreenConfig, PAD_16, PAD_32, PAD_64, PAD_8, RADIUS_8},
    widgets::Square,
    {
        ClickResult, CursorIcon, PatternData, Rectangle, Vertex, NO_RADIUS, PAD_2,
        TOOLBAR_THICKNESS, TOOLBAR_Y,
    },
};

const ICON_SIZE: f32 = 20.0;

pub fn draw(
    screen_config: &ScreenConfig,
    patterns: &[PatternData],
    active_pattern_id: usize,
    mouse_state: &MouseState,
    sequencer_is_open: bool,
    tray_width: f32,
    out: &mut Vec<Vertex>,
) -> (
    Vec<TextItem>,
    ClickResult,
    CursorIcon,
    IconDraw,
    Option<Tooltip>,
) {
    // setup

    let mut text_items: Vec<TextItem> = Vec::new();
    let mut click_result = ClickResult::None;
    let mut cursor_icon = CursorIcon::Default;
    let mut tooltip = None;

    // patterns tray
    let pattern_tray = Rectangle {
        x: screen_config.width as f32 - 128.0,
        y: TOOLBAR_Y,
        width: tray_width,
        height: screen_config.height as f32 - TOOLBAR_THICKNESS,
    };
    pattern_tray.draw(screen_config, PEBBLE, NO_RADIUS, out);

    let w_divider = Rectangle {
        x: pattern_tray.x,
        y: pattern_tray.y,
        width: 1.0,
        height: pattern_tray.height,
    };
    w_divider.draw(screen_config, LL_GRAY, NO_RADIUS, out);

    if pattern_tray.is_hovered_left_edge(mouse_state.x, mouse_state.y) {
        cursor_icon = CursorIcon::ColResize
    }
    // title
    text_items.push(draw_title("Patterns", (pattern_tray.x, pattern_tray.y)));

    // add pattern button
    let add_pattern_button = Square {
        x: screen_config.width as f32 - PAD_32,
        y: pattern_tray.y + PAD_8,
        size: ICON_SIZE,
    };
    add_pattern_button.draw(
        screen_config,
        icon_color(
            &add_pattern_button,
            mouse_state.x,
            mouse_state.y,
            mouse_state.left_click_held,
        ),
        RADIUS_8,
        out,
    );
    if add_pattern_button.is_hovered(mouse_state.x, mouse_state.y) {
        cursor_icon = CursorIcon::Pointer;
        if mouse_state.left_clicked {
            click_result = ClickResult::CreatePattern;
        }
    }

    let add_icon = IconDraw {
        name: "add",
        x: add_pattern_button.x,
        y: add_pattern_button.y,
        width: ICON_SIZE,
        height: ICON_SIZE,
        tooltip: Tooltip {
            text: Some("Add pattern"),
            x: add_pattern_button.x - PAD_64 - PAD_64 - PAD_8,
            y: add_pattern_button.y,
        },
    };
    if add_icon.is_hovered(mouse_state.x, mouse_state.y) {
        tooltip = Some(add_icon.tooltip.clone());
    }

    for (i, pattern) in patterns.iter().enumerate() {
        // draw the shape
        let pattern_button = Rectangle {
            x: pattern_tray.x + PAD_16,
            y: PATTERN_TRAY_HEADER_MARGIN + (PATTERN_TRAY_ITEM_GAP * i as f32) + PAD_32,
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
            indicator.draw(
                screen_config,
                crate::graphics::color::ORANGE,
                [7.0, 7.0, 7.0, 7.0],
                out,
            );
        }
        let pattern_button_color = if pattern_button.is_hovered(mouse_state.x, mouse_state.y) {
            LIGHT_GRAY_HOVER
        } else {
            LIGHT_GRAY
        };
        pattern_button.draw(
            screen_config,
            pattern_button_color,
            [4.0, 4.0, 4.0, 4.0],
            out,
        );

        // handle interaction
        if pattern_button.is_hovered(mouse_state.x, mouse_state.y) {
            cursor_icon = CursorIcon::Pointer;
            if mouse_state.left_clicked {
                click_result = ClickResult::SelectPattern(pattern.id);
            }
            if mouse_state.left_double_clicked && !sequencer_is_open {
                click_result = ClickResult::ToggleSequencerWindow;
            }
            if mouse_state.right_clicked {
                click_result =
                    ClickResult::OpenPatternMenu(pattern_button.x, pattern_button.y, pattern.id);
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

    (text_items, click_result, cursor_icon, add_icon, tooltip)
}
