use winit::window::CursorIcon;

use crate::{
    app::{MouseState, click::ClickResult},
    graphics::{
        FrameDrawContext,
        components::toolbar::{IconRow, TOOLTIP_MARGIN},
        geometry::{DrawResponse, Rectangle, icon_border},
        icons::{DEFAULT_TOOLTIP_WIDTH, IconDraw, Tooltip},
        mini_window::{InteractionResult, MiniWindow},
        primitives::{NO_RADIUS, PAD_4, PAD_16, PAD_32, RADIUS_4, ScreenConfig, VertexSink},
    },
};

#[derive(PartialEq, Debug)]
pub enum PlaylistTool {
    Select,
    Rectangle,
    Paint,
    Mute,
}

fn draw_tool_button(
    tool: PlaylistTool,
    active_tool: &PlaylistTool,
    rect: Rectangle,
    mouse_state: &MouseState,
    screen_config: &ScreenConfig,
    out: &mut impl VertexSink,
) -> (DrawResponse, InteractionResult) {
    let color = if tool == *active_tool {
        screen_config.palette.secondary.shade(4)
    } else {
        screen_config.palette.neutral.shade(3)
    };

    let response =
        rect.draw_style()
            .interactive(Some(mouse_state))
            .draw(screen_config, color, RADIUS_4, out);

    let mut interaction = InteractionResult::default();
    if response.hovered {
        interaction.cursor = CursorIcon::Pointer;
        if mouse_state.left_clicked {
            interaction.click = ClickResult::ChangePlaylistTool(tool);
        }
    }

    (response, interaction)
}

pub fn draw(
    window: &MiniWindow,
    screen_config: &ScreenConfig,
    mouse_state: &MouseState,
    active_tool: &PlaylistTool,
    frame: &mut FrameDrawContext<'_>,
) -> InteractionResult {
    let mut interaction = InteractionResult {
        cursor: CursorIcon::Default,
        click: ClickResult::None,
    };
    let background = Rectangle::new(
        window.x + PAD_16,
        window.y + PAD_4,
        window.width - PAD_32 * 2.0,
        PAD_16 * 2.0,
    )
    .draw_style()
    .bordered(Some(icon_border(screen_config.palette)))
    .draw(
        screen_config,
        screen_config.palette.neutral.shade(1),
        NO_RADIUS,
        frame,
    );

    let mut window_icons = IconRow {
        x: background.x + PAD_4,
        y: background.y + PAD_4,
        size: PLAYLIST_ICON_SIZE,
        gap: 32.0,
    };

    const PLAYLIST_ICON_SIZE: f32 = 24.0;

    const TOOLS: [(PlaylistTool, &str, &str); 4] = [
        (PlaylistTool::Select, "PlaylistSelect", "Select"),
        (PlaylistTool::Paint, "PlaylistPaint", "Paint"),
        (PlaylistTool::Rectangle, "PlaylistRectangle", "Rectangle"),
        (PlaylistTool::Mute, "Mute", "Mute"),
    ];

    for (tool, icon_name, tooltip_text) in TOOLS {
        let (response, tool_interaction) = draw_tool_button(
            tool,
            active_tool,
            window_icons.next_icon(),
            mouse_state,
            screen_config,
            frame,
        );
        interaction = interaction.or(tool_interaction);

        let icon = IconDraw {
            name: icon_name,
            x: response.x,
            y: response.y,
            width: response.width,
            height: response.height,
            tooltip: Tooltip {
                text: Some(tooltip_text.to_string()),
                x: response.x,
                y: response.y + TOOLTIP_MARGIN,
                width: DEFAULT_TOOLTIP_WIDTH,
            },
        };
        frame.draw_icon(&icon);
        if !mouse_state.left_click_held && icon.is_hovered(mouse_state.x, mouse_state.y) {
            frame.set_tooltip_if_some(Some(icon.tooltip.clone()));
        }
    }

    interaction
}
