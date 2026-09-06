use crate::{
    app::MouseState,
    graphics::{
        color::DARK_GRAY,
        components::toolbar::{IconRow, TOOLTIP_MARGIN},
        font::TextItem,
        geometry::{Rectangle, ICON_BORDER},
        icons::{IconDraw, Tooltip, DEFAULT_TOOLTIP_WIDTH},
        mini_window::MiniWindow,
        primitives::{ScreenConfig, Vertex, NO_RADIUS, PAD_16, PAD_32, PAD_4, RADIUS_4},
    },
};

pub fn draw(
    window: &MiniWindow,
    screen_config: &ScreenConfig,
    mouse_state: &MouseState,
) -> (Vec<Vertex>, Vec<TextItem>, Vec<IconDraw>) {
    let mut verts = vec![];
    let texts = vec![];
    let background = Rectangle::new(
        window.x + PAD_16,
        window.y + PAD_4,
        window.width - PAD_32 * 2.0,
        PAD_16 * 2.0,
    )
    .draw_style()
    .bordered(Some(ICON_BORDER))
    .draw(screen_config, DARK_GRAY, NO_RADIUS, &mut verts);

    const PLAYLIST_ICON_SIZE: f32 = 24.0;

    let mut window_icons = IconRow {
        x: background.x + PAD_4,
        y: background.y + PAD_4,
        size: PLAYLIST_ICON_SIZE,
        gap: 32.0,
    };

    let select_toggle = window_icons
        .next()
        .draw_style()
        .interactive(Some(mouse_state))
        .draw(screen_config, DARK_GRAY, RADIUS_4, &mut verts);

    let paint_toggle = window_icons
        .next()
        .draw_style()
        .interactive(Some(mouse_state))
        .draw(screen_config, DARK_GRAY, RADIUS_4, &mut verts);

    let rectangle_toggle = window_icons
        .next()
        .draw_style()
        .interactive(Some(mouse_state))
        .draw(screen_config, DARK_GRAY, RADIUS_4, &mut verts);

    let mute_toggle = window_icons
        .next()
        .draw_style()
        .interactive(Some(mouse_state))
        .draw(screen_config, DARK_GRAY, RADIUS_4, &mut verts);

    let icons: Vec<IconDraw> = vec![
        IconDraw {
            name: "PlaylistSelect",
            x: select_toggle.x,
            y: select_toggle.y,
            width: select_toggle.width,
            height: select_toggle.height,
            tooltip: Tooltip {
                text: Some("Select".to_string()),
                x: select_toggle.x,
                y: select_toggle.y + TOOLTIP_MARGIN,
                width: DEFAULT_TOOLTIP_WIDTH,
            },
        },
        IconDraw {
            name: "PlaylistPaint",
            x: paint_toggle.x,
            y: paint_toggle.y,
            width: paint_toggle.width,
            height: paint_toggle.height,
            tooltip: Tooltip {
                text: Some("{aint".to_string()),
                x: paint_toggle.x,
                y: paint_toggle.y + TOOLTIP_MARGIN,
                width: DEFAULT_TOOLTIP_WIDTH,
            },
        },
        IconDraw {
            name: "PlaylistRectangle",
            x: rectangle_toggle.x,
            y: rectangle_toggle.y,
            width: rectangle_toggle.width,
            height: rectangle_toggle.height,
            tooltip: Tooltip {
                text: Some("{aint".to_string()),
                x: rectangle_toggle.x,
                y: rectangle_toggle.y + TOOLTIP_MARGIN,
                width: DEFAULT_TOOLTIP_WIDTH,
            },
        },
        IconDraw {
            name: "Mute",
            x: mute_toggle.x,
            y: mute_toggle.y,
            width: mute_toggle.width,
            height: mute_toggle.height,
            tooltip: Tooltip {
                text: Some("mute".to_string()),
                x: mute_toggle.x,
                y: mute_toggle.y + TOOLTIP_MARGIN,
                width: DEFAULT_TOOLTIP_WIDTH,
            },
        },
    ];

    return (verts, texts, icons);
}
