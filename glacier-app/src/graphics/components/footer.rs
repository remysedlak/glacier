use winit::window::CursorIcon;

use crate::{
    app::{click::ClickResult, MouseState},
    graphics::{
        font::{Font::Mono, TextItem, BODY},
        geometry::Rectangle,
        icons::{IconDraw, Tooltip},
        mini_window::InteractionResult,
        primitives::{NO_RADIUS, PAD_16, PAD_4, PAD_8, RADIUS_8},
        FrameDrawContext,
    },
};

pub const FOOTER_Y_HEIGHT: f32 = 32.0;
pub const FPS_COUNTER_X_OFFSET: f32 = 80.0;
pub const FOOTER_ICON_SIZE: f32 = 20.0;

/// Draw the app footer. Contains project metadata and frame rate label
pub fn draw(
    frame: &mut FrameDrawContext<'_>,
    path: &str,
    frame_rate: f32,
    mouse_state: &MouseState,
) {
    // setup

    let screen_config = *frame.screen_config();
    let mut tooltip: Option<Tooltip> = None;
    let mut interaction = InteractionResult::default();

    let footer = Rectangle {
        x: 0.0,
        y: screen_config.height as f32 - FOOTER_Y_HEIGHT,
        width: screen_config.width as f32,
        height: FOOTER_Y_HEIGHT,
    };
    footer.draw(
        &screen_config,
        screen_config.palette.neutral.shade(0),
        NO_RADIUS,
        frame,
    );

    // click button to find project file
    let left_panel_button = Rectangle::new(
        PAD_8,
        screen_config.height as f32 - FOOTER_Y_HEIGHT + PAD_8,
        FOOTER_ICON_SIZE,
        FOOTER_ICON_SIZE,
    )
    .draw_style()
    .interactive(Some(mouse_state))
    .draw(
        &screen_config,
        screen_config.palette.neutral.shade(0),
        RADIUS_8,
        frame,
    );

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
    frame.draw_icon(&left_panel_icon);

    let border = Rectangle::new(
        left_panel_button.x + left_panel_button.width + PAD_8 + PAD_4,
        footer.y + 10.0,
        1.0,
        footer.height - PAD_16,
    );
    border.draw(
        &screen_config,
        screen_config.palette.neutral.shade(7),
        NO_RADIUS,
        frame,
    );

    // click button to find project file
    let path_button = Rectangle::new(
        border.x + PAD_8 + PAD_4,
        screen_config.height as f32 - FOOTER_Y_HEIGHT + PAD_8,
        FOOTER_ICON_SIZE,
        FOOTER_ICON_SIZE,
    )
    .draw_style()
    .interactive(Some(mouse_state))
    .draw(
        &screen_config,
        screen_config.palette.neutral.shade(0),
        RADIUS_8,
        frame,
    );

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
            interaction.click = ClickResult::OpenTrackFileLocation(path.to_owned());
        }
    }
    frame.draw_icon(&music_folder_icon);

    // display frames per second
    let fps_text = TextItem {
        text: (frame_rate as u32).to_string(),
        x: screen_config.width as f32 - FPS_COUNTER_X_OFFSET,
        y: screen_config.height as f32 - FOOTER_Y_HEIGHT + PAD_8,
        size: BODY,
        color: screen_config.palette.primary.shade(5),
        font: Mono,
    };
    frame.draw_texts(&[fps_text]);
    frame.add_interaction(interaction);
    frame.set_tooltip_if_some(tooltip);
}
