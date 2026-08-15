//! Modals are popups that appear in the center of the screen, with a dim overlay behind them.
use crate::app::MouseState;
use crate::graphics::{
    color::{DARK_GRAY, SURFACE, WHITE},
    font::{TextItem, ROBOTO},
    geometry::Rectangle,
    primitives::{ScreenConfig, Vertex, PAD_16, PAD_4, PAD_8, RADIUS_4, RADIUS_8},
    ClickResult, CursorIcon,
};

const MODAL_HEIGHT: f32 = 256.0;
const MODAL_WIDTH: f32 = 512.0;
const BUTTON_WIDTH: f32 = 96.0;
const BUTTON_HEIGHT: f32 = 32.0;
const BUTTON_GAP: f32 = 8.0;

/// Draw the "save changes before closing?" modal.
/// Returns the text to render, the click result (which button, if any, was pressed),
/// and the cursor icon to show.
pub fn draw(
    screen_config: &ScreenConfig,
    mouse_state: &MouseState,
    out: &mut Vec<Vertex>,
) -> (Vec<TextItem>, ClickResult, CursorIcon) {
    let mut text_items: Vec<TextItem> = Vec::new();
    let mut click_result = ClickResult::None;
    let mut cursor_icon = CursorIcon::Default;

    // centered modal box
    let modal_background = Rectangle {
        x: (screen_config.width as f32 / 2.0) - MODAL_WIDTH / 2.0,
        y: (screen_config.height as f32 / 2.0) - MODAL_HEIGHT / 2.0,
        height: MODAL_HEIGHT,
        width: MODAL_WIDTH,
    };
    modal_background.draw(screen_config, SURFACE, RADIUS_8, out);

    text_items.push(TextItem {
        text: "Save changes before closing?".to_string(),
        x: modal_background.x + PAD_16,
        y: modal_background.y + PAD_16,
        size: 16.0,
        color: WHITE,
        font: ROBOTO,
    });

    text_items.push(TextItem {
        text: "Unsaved changes will be lost if you don't save.".to_string(),
        x: modal_background.x + PAD_16,
        y: modal_background.y + PAD_16 + 28.0,
        size: 13.0,
        color: WHITE,
        font: ROBOTO,
    });

    // buttons sit along the bottom edge, right-aligned
    let button_y = modal_background.y + MODAL_HEIGHT - PAD_16 - BUTTON_HEIGHT;
    let cancel_button = Rectangle {
        x: modal_background.x + MODAL_WIDTH - PAD_16 - BUTTON_WIDTH,
        y: button_y,
        width: BUTTON_WIDTH,
        height: BUTTON_HEIGHT,
    };
    let discard_button = Rectangle {
        x: cancel_button.x - BUTTON_GAP - BUTTON_WIDTH,
        y: button_y,
        width: BUTTON_WIDTH,
        height: BUTTON_HEIGHT,
    };
    let save_button = Rectangle {
        x: discard_button.x - BUTTON_GAP - BUTTON_WIDTH,
        y: button_y,
        width: BUTTON_WIDTH,
        height: BUTTON_HEIGHT,
    };

    if save_button.draw_interactive(screen_config, DARK_GRAY, mouse_state, RADIUS_4, out) {
        cursor_icon = CursorIcon::Pointer;
        if mouse_state.left_clicked {
            click_result = ClickResult::ModalConfirmSaveAndExit;
        }
    }
    if discard_button.draw_interactive(screen_config, DARK_GRAY, mouse_state, RADIUS_4, out) {
        cursor_icon = CursorIcon::Pointer;
        if mouse_state.left_clicked {
            click_result = ClickResult::ModalConfirmDiscardAndExit;
        }
    }
    if cancel_button.draw_interactive(screen_config, DARK_GRAY, mouse_state, RADIUS_4, out) {
        cursor_icon = CursorIcon::Pointer;
        if mouse_state.left_clicked {
            click_result = ClickResult::ModalCancelExit;
        }
    }

    text_items.push(TextItem {
        text: "Save".to_string(),
        x: save_button.x + PAD_8,
        y: save_button.y + PAD_4,
        size: 14.0,
        color: WHITE,
        font: ROBOTO,
    });
    text_items.push(TextItem {
        text: "Discard".to_string(),
        x: discard_button.x + PAD_8,
        y: discard_button.y + PAD_4,
        size: 14.0,
        color: WHITE,
        font: ROBOTO,
    });
    text_items.push(TextItem {
        text: "Cancel".to_string(),
        x: cancel_button.x + PAD_8,
        y: cancel_button.y + PAD_4,
        size: 14.0,
        color: WHITE,
        font: ROBOTO,
    });

    (text_items, click_result, cursor_icon)
}
