use crate::app::MouseState;
use crate::graphics::color::{DARK_GRAY, ORANGE, SURFACE, SURFACE_HOVER, WHITE};
use crate::graphics::primitives::{RenameState, RenameTarget, PAD_4};
use crate::graphics::{
    components::side_panel::*,
    font::{TextItem, ROBOTO},
    icons::{IconDraw, Tooltip},
    primitives::{ScreenConfig, PAD_32, PAD_64, PAD_8, RADIUS_8},
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
    selected_pattern_id: Option<u32>,
    mouse_state: &MouseState,
    sequencer_is_open: bool,
    tray_width: f32,
    renaming: &Option<RenameState>,
    rename_cursor_offset: Option<f32>,
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
    pattern_tray.draw(screen_config, SURFACE, NO_RADIUS, out);

    let w_divider = Rectangle {
        x: pattern_tray.x,
        y: pattern_tray.y,
        width: 1.0,
        height: pattern_tray.height,
    };
    w_divider.draw_interactive(screen_config, DARK_GRAY, mouse_state, NO_RADIUS, out);

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
    let add_button_hovered =
        add_pattern_button.draw_interactive(screen_config, DARK_GRAY, mouse_state, RADIUS_8, out);
    if add_button_hovered {
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

    // single pass: each pattern's row position is computed once and
    // reused for its button, selection indicator, and label, so they
    // can never drift out of sync with each other.
    for (i, pattern) in patterns.iter().enumerate() {
        let row_y = PATTERN_TRAY_HEADER_MARGIN + (PATTERN_TRAY_ITEM_GAP * i as f32) + PAD_32;

        let pattern_button = Rectangle {
            x: pattern_tray.x + PAD_4,
            y: row_y,
            width: pattern_tray.width - PAD_8,
            height: PATTERN_TRAY_ITEM_HEIGHT,
        };

        let hovered = pattern_button.is_hovered(mouse_state.x, mouse_state.y);
        let pattern_button_color = if hovered { SURFACE_HOVER } else { SURFACE };
        pattern_button.draw(
            screen_config,
            pattern_button_color,
            [4.0, 4.0, 4.0, 4.0],
            out,
        );

        // handle interaction (reuses the `hovered` computed above instead
        // of calling is_hovered a second time)
        if hovered {
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

        if Some(pattern.id as u32) == selected_pattern_id {
            let indicator = Rectangle {
                x: pattern_button.x,
                y: row_y,
                width: 4.0,
                height: PATTERN_TRAY_ITEM_HEIGHT,
            };
            indicator.draw(screen_config, ORANGE, [7.0, 7.0, 7.0, 7.0], out);
        }

        let is_being_renamed = matches!(
            renaming,
            Some(r) if r.target == RenameTarget::Pattern(pattern.id)
        );

        let pattern_label = if is_being_renamed {
            renaming.as_ref().unwrap().edited_name.clone()
        } else {
            pattern.name.clone()
        };

        text_items.push(TextItem {
            text: pattern_label,
            x: screen_config.width as f32 - PATTERN_TRAY_ITEM_WIDTH,
            y: row_y + PAD_2,
            size: 14.0,
            color: WHITE,
            font: ROBOTO,
        });

        if is_being_renamed {
            if let Some(offset) = rename_cursor_offset {
                let cursor_rect = Rectangle {
                    x: screen_config.width as f32 - PATTERN_TRAY_ITEM_WIDTH + offset, // text_x = wherever the label's x already is
                    y: row_y + PAD_2,
                    width: 1.5,
                    height: 14.0, // match font size
                };
                cursor_rect.draw(screen_config, WHITE, NO_RADIUS, out);
            }
        }
    }

    (text_items, click_result, cursor_icon, add_icon, tooltip)
}
