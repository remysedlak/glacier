use winit::window::CursorIcon;

use crate::{
    app::{click::ClickResult, MouseState},
    graphics::{
        color::{BLACK, ORANGE, WHITE},
        font::{TextItem, BODY, MONOSPACED},
        geometry::Rectangle,
        icons::{IconDraw, Tooltip},
        mini_window::InteractionResult,
        primitives::{ScreenConfig, Vertex, NO_RADIUS, PAD_16, PAD_4, PAD_8, RADIUS_8},
    },
};

pub const FOOTER_Y_HEIGHT: f32 = 32.0;
pub const FPS_COUNTER_X_OFFSET: f32 = 80.0;
pub const FOOTER_ICON_SIZE: f32 = 20.0;

/// Draw the app footer. Contains project metadata and frame rate label
pub fn draw(
    screen_config: &ScreenConfig,
    path: &String,
    frame_rate: f32,
    mouse_state: &MouseState,
    out: &mut Vec<Vertex>,
) -> (
    Vec<TextItem>,
    Vec<IconDraw>,
    Option<Tooltip>,
    InteractionResult,
) {
    // setup

    let mut text_items: Vec<TextItem> = Vec::new();
    let mut tooltip: Option<Tooltip> = None;
    let mut icons: Vec<IconDraw> = Vec::new();
    let mut interaction = InteractionResult::default();

    let footer = Rectangle {
        x: 0.0,
        y: screen_config.height as f32 - FOOTER_Y_HEIGHT,
        width: screen_config.width as f32,
        height: FOOTER_Y_HEIGHT,
    };
    footer.draw(screen_config, BLACK, NO_RADIUS, out);

    // click button to find project file
    let left_panel_button = Rectangle::new(
        PAD_8,
        screen_config.height as f32 - FOOTER_Y_HEIGHT + PAD_8,
        FOOTER_ICON_SIZE,
        FOOTER_ICON_SIZE,
    )
    .draw_style()
    .interactive(Some(mouse_state))
    .disabled()
    .draw(screen_config, BLACK, RADIUS_8, out);

    let left_panel_icon = IconDraw {
        name: "left_sidepanel",
        x: left_panel_button.x,
        y: left_panel_button.y,
        width: FOOTER_ICON_SIZE,
        height: FOOTER_ICON_SIZE,
        tooltip: Tooltip {
            text: Some("Open Track Sidebar".to_string()),
            x: left_panel_button.x + PAD_16,
            y: left_panel_button.y - PAD_16,
            width: 128.0,
        },
    };

    if left_panel_button.hovered {
        tooltip = Some(left_panel_icon.tooltip.clone());
        interaction.cursor = CursorIcon::Pointer;
        if mouse_state.left_clicked {
            interaction.click = ClickResult::ToggleTrackTray;
        }
    }
    icons.push(left_panel_icon);

    let border = Rectangle::new(
        left_panel_button.x + left_panel_button.width + PAD_8 + PAD_4,
        footer.y + 10.0,
        1.0,
        footer.height - PAD_16,
    );
    border.draw(screen_config, WHITE, NO_RADIUS, out);

    // click button to find project file
    let path_button = Rectangle::new(
        border.x + PAD_8 + PAD_4,
        screen_config.height as f32 - FOOTER_Y_HEIGHT + PAD_8,
        FOOTER_ICON_SIZE,
        FOOTER_ICON_SIZE,
    )
    .draw_style()
    .interactive(Some(mouse_state))
    .disabled()
    .draw(screen_config, BLACK, RADIUS_8, out);

    let music_folder_icon = IconDraw {
        name: "music_dir",
        x: path_button.x,
        y: path_button.y,
        width: FOOTER_ICON_SIZE,
        height: FOOTER_ICON_SIZE,
        tooltip: Tooltip {
            text: Some("Open Project in File Manager".to_string()),
            x: left_panel_button.x + PAD_16,
            y: left_panel_button.y - PAD_16,
            width: 280.0,
        },
    };
    if path_button.hovered {
        tooltip = Some(music_folder_icon.tooltip.clone());
        interaction.cursor = CursorIcon::Pointer;
        if mouse_state.left_clicked {
            interaction.click = ClickResult::OpenTrackFileLocation(path.to_string());
        }
    }
    icons.push(music_folder_icon);

    // display frames per second
    text_items.push(TextItem {
        text: (frame_rate as u32).to_string(),
        x: screen_config.width as f32 - FPS_COUNTER_X_OFFSET,
        y: screen_config.height as f32 - FOOTER_Y_HEIGHT + PAD_8,
        size: BODY,
        color: ORANGE,
        font: MONOSPACED,
    });
    (text_items, icons, tooltip, interaction)
}
