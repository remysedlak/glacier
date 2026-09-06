use winit::window::CursorIcon;

use crate::{
    app::{click::ClickResult, MouseState},
    graphics::{
        color::{DARK_GRAY, ORANGE},
        components::toolbar::{IconRow, TOOLTIP_MARGIN},
        font::TextItem,
        geometry::{DrawResponse, Rectangle, ICON_BORDER},
        icons::{IconDraw, Tooltip, DEFAULT_TOOLTIP_WIDTH},
        mini_window::{InteractionResult, MiniWindow},
        primitives::{ScreenConfig, Vertex, NO_RADIUS, PAD_16, PAD_32, PAD_4, RADIUS_4},
    },
};

#[derive(PartialEq)]
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
    out: &mut Vec<Vertex>,
) -> (DrawResponse, InteractionResult) {
    let color = if tool == *active_tool {
        ORANGE
    } else {
        DARK_GRAY
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
) -> (Vec<Vertex>, Vec<TextItem>, Vec<IconDraw>, InteractionResult) {
    let mut verts = vec![];
    let texts = vec![];
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
    .bordered(Some(ICON_BORDER))
    .draw(screen_config, DARK_GRAY, NO_RADIUS, &mut verts);

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

    let mut icons = Vec::new();
    for (tool, icon_name, tooltip_text) in TOOLS {
        let (response, tool_interaction) = draw_tool_button(
            tool,
            active_tool,
            window_icons.next(),
            mouse_state,
            screen_config,
            &mut verts,
        );
        interaction = interaction.or(tool_interaction);

        icons.push(IconDraw {
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
        });
    }

    return (verts, texts, icons, interaction);
}
