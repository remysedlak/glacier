use winit::window::CursorIcon;

use crate::{
    app::{click::ClickResult, MouseState},
    graphics::{
        color::{BLACK, DARK_BLUE, DARK_GRAY, LL_GRAY, ORANGE, WHITE},
        components::toolbar::ICON_SIZE,
        font::{TextItem, BODY, MONOSPACED, ROBOTO},
        geometry::Rectangle,
        icons::{IconDraw, Tooltip},
        primitives::{
            ScreenConfig, Vertex, NO_RADIUS, PAD_16, PAD_2, PAD_32, PAD_4, PAD_8, RADIUS_8,
        },
    },
};

pub const FOOTER_Y_HEIGHT: f32 = 32.0;
pub const FPS_COUNTER_X_OFFSET: f32 = 80.0;

/// Draw the app footer. Contains project metadata and frame rate label
pub fn draw(
    screen_config: &ScreenConfig,
    path: &String,
    frame_rate: f32,
    mouse_state: &MouseState,
    out: &mut Vec<Vertex>,
) -> (
    Vec<TextItem>,
    ClickResult,
    IconDraw,
    Option<Tooltip>,
    CursorIcon,
) {
    // setup
    let mut click_result = ClickResult::None;
    let mut text_items: Vec<TextItem> = Vec::new();
    let mut tooltip: Option<Tooltip> = None;
    let mut cursor_icon = CursorIcon::Default;

    let footer = Rectangle {
        x: 0.0,
        y: screen_config.height as f32 - FOOTER_Y_HEIGHT,
        width: screen_config.width as f32,
        height: FOOTER_Y_HEIGHT,
    };
    footer.draw(screen_config, BLACK, NO_RADIUS, out);

    // click button to find project file
    let button = Rectangle::new(
        PAD_4,
        screen_config.height as f32 - FOOTER_Y_HEIGHT + PAD_8,
        20.0,
        16.0,
    )
    .draw_style()
    .interactive(Some(mouse_state))
    .disabled()
    .draw(screen_config, BLACK, RADIUS_8, out);

    let icon = IconDraw {
        name: "music_dir",
        x: button.x,
        y: button.y,
        width: button.width,
        height: button.height,
        tooltip: Tooltip {
            text: Some("Open Project in File Manager".to_string()),
            x: button.x + PAD_16,
            y: button.y - PAD_16,
            width: 280.0,
        },
    };

    if button.hovered {
        tooltip = Some(icon.tooltip.clone());
        cursor_icon = CursorIcon::Pointer;
        if mouse_state.left_clicked {
            click_result = ClickResult::OpenTrackFileLocation(path.to_string());
        }
    }

    let border = Rectangle::new(
        button.x + PAD_32,
        footer.y + PAD_8,
        1.0,
        footer.height - PAD_16,
    )
    .draw(screen_config, WHITE, NO_RADIUS, out);

    // display frames per second
    text_items.push(TextItem {
        text: (frame_rate as u32).to_string(),
        x: screen_config.width as f32 - FPS_COUNTER_X_OFFSET,
        y: screen_config.height as f32 - FOOTER_Y_HEIGHT + PAD_8,
        size: BODY,
        color: ORANGE,
        font: MONOSPACED,
    });
    (text_items, click_result, icon, tooltip, cursor_icon)
}
