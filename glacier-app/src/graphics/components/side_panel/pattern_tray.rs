use crate::app::click::ClickResult;
use crate::app::MouseState;
use crate::graphics::icons::DEFAULT_TOOLTIP_WIDTH;
use crate::graphics::mini_window::InteractionResult;
use crate::graphics::{
    color::{DARK_GRAY, ORANGE, SURFACE, SURFACE_HOVER, WHITE},
    components::{
        side_panel::*,
        toolbar::{TOOLBAR_THICKNESS, TOOLBAR_Y},
    },
    font::{TextItem, ROBOTO},
    icons::{IconDraw, Tooltip},
    primitives::{RenameState, RenameTarget, ScreenConfig, PAD_32, PAD_4, PAD_64, PAD_8, RADIUS_8},
    {CursorIcon, PatternData, Rectangle, Vertex, NO_RADIUS, PAD_2},
};
use crate::project::PatternID;

const ICON_SIZE: f32 = 20.0;

pub fn draw(
    screen_config: &ScreenConfig,
    patterns: &[PatternData],
    selected_pattern_id: Option<PatternID>,
    mouse_state: &MouseState,
    sequencer_is_open: bool,
    tray_width: f32,
    renaming: &Option<RenameState>,
    rename_cursor_offset: Option<f32>,
    out: &mut Vec<Vertex>,
) -> (Vec<TextItem>, InteractionResult, IconDraw, Option<Tooltip>) {
    // setup
    let mut text_items: Vec<TextItem> = Vec::new();
    let mut interaction = InteractionResult::default();

    let mut tooltip = None;

    // patterns tray
    let pattern_tray = Rectangle {
        x: screen_config.width as f32 - 128.0,
        y: TOOLBAR_Y,
        width: tray_width,
        height: screen_config.height as f32 - TOOLBAR_THICKNESS,
    };
    pattern_tray.draw(screen_config, SURFACE, NO_RADIUS, out);

    let _w_divider = Rectangle::new(pattern_tray.x, pattern_tray.y, 1.0, pattern_tray.height)
        .draw_style()
        .interactive(Some(mouse_state))
        .draw(screen_config, DARK_GRAY, NO_RADIUS, out);

    if pattern_tray.is_hovered_left_edge(mouse_state.x, mouse_state.y) {
        interaction.cursor = CursorIcon::ColResize
    }

    // title
    text_items.push(draw_title("Patterns", (pattern_tray.x, pattern_tray.y)));

    // add pattern button
    let add_pattern_button = Rectangle::square(
        screen_config.width as f32 - PAD_32,
        pattern_tray.y + PAD_8,
        ICON_SIZE,
    )
    .draw_style()
    .interactive(Some(mouse_state))
    .draw(screen_config, DARK_GRAY, RADIUS_8, out);

    if add_pattern_button.hovered {
        interaction.cursor = CursorIcon::Pointer;
        if mouse_state.left_clicked {
            interaction.click = ClickResult::CreatePattern;
        }
    }

    let add_icon = IconDraw {
        name: "add",
        x: add_pattern_button.x,
        y: add_pattern_button.y,
        width: ICON_SIZE,
        height: ICON_SIZE,
        tooltip: Tooltip {
            text: Some("Add pattern".to_string()),
            x: add_pattern_button.x - PAD_64 - PAD_64 - PAD_8,
            y: add_pattern_button.y,
            width: DEFAULT_TOOLTIP_WIDTH,
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
            interaction.cursor = CursorIcon::Pointer;
            if mouse_state.left_clicked {
                interaction.click = ClickResult::SelectPattern(pattern.id);
            }
            if mouse_state.left_double_clicked && !sequencer_is_open {
                interaction.click = ClickResult::ToggleSequencerWindow;
            }
            if mouse_state.right_clicked {
                interaction.click =
                    ClickResult::OpenPatternMenu(pattern.id, pattern_button.x, pattern_button.y);
            }
        }

        if Some(pattern.id) == selected_pattern_id {
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

    (text_items, interaction, add_icon, tooltip)
}
