use crate::app::{click::ClickResult, MouseState};
use crate::graphics::icons::DEFAULT_TOOLTIP_WIDTH;
use crate::graphics::mini_window::InteractionResult;
use crate::graphics::{
    color::{DARK_GRAY, MINI_WINDOW_BACKGROUND, WHITE},
    components::toolbar::TOOLTIP_MARGIN,
    icons::{IconDraw, Tooltip},
    mini_window::{MiniWindow, TITLEBAR_HEIGHT},
    primitives::{ScreenConfig, Vertex, NO_RADIUS, PAD_16, PAD_8, RADIUS_4},
    {Rectangle, TextItem},
};
use crate::project::Track;
use winit::window::CursorIcon;

const TRACK_GRAPHICS_WIDTH: f32 = 200.0;
const TRACK_GRAPHICS_HEIGHT: f32 = 128.0;
const TRACK_GRAPHICS_HEIGHT_HALF: f32 = 128.0 / 2.0;

/// Draw a Mini Window that contains the track information and audio tools
pub fn draw(
    window: &MiniWindow,
    mouse_state: &MouseState,
    screen_config: &ScreenConfig,
    track: &Track,
    out: &mut Vec<Vertex>,
) -> (
    Vec<TextItem>,
    Vec<IconDraw>,
    InteractionResult,
    Option<Tooltip>,
) {
    // setup
    let mut text_items: Vec<TextItem> = Vec::new();
    let mut icons: Vec<IconDraw> = Vec::new();
    let mut interaction = InteractionResult {
        cursor: CursorIcon::Default,
        click: ClickResult::None,
    };
    let mut tooltip: Option<Tooltip> = None;
    // window background
    let window_background = window.background();
    window_background.draw(
        screen_config,
        MINI_WINDOW_BACKGROUND,
        [0.0, 16.0, 0.0, 16.0],
        out,
    );

    // titlebar
    let (titlebar_texts, titlebar_interaction) = window.title_bar(
        &format!("Track: {}", track.data.name),
        screen_config,
        mouse_state,
        out,
    );
    interaction = interaction.or(titlebar_interaction);
    text_items.push(titlebar_texts);

    // draw background of wave form for track
    let graphics_y = (window.y) + TITLEBAR_HEIGHT - PAD_16;
    let center_y = graphics_y + TRACK_GRAPHICS_HEIGHT / 2.0;
    let track_wave_background = Rectangle {
        x: (window.x + window.width) - PAD_16 - TRACK_GRAPHICS_WIDTH,
        y: graphics_y,
        width: TRACK_GRAPHICS_WIDTH,
        height: TRACK_GRAPHICS_HEIGHT,
    };
    track_wave_background.draw(screen_config, DARK_GRAY, NO_RADIUS, out);

    let samples_averaged: Vec<f32> = track
        .samples
        .chunks(2)
        .map(|pair| (pair[0] + pair[1]) / 2.0)
        .collect::<Vec<f32>>();
    let sample_stride = samples_averaged.len() / TRACK_GRAPHICS_WIDTH as usize;

    // 200 pixel columns for 200 pixel graphics
    for pixel_column in 0..199 {
        // get the first and last position of the stride
        let start = pixel_column * sample_stride;
        let end = (start + sample_stride).min(samples_averaged.len());
        // using  the start and end range,  find the highest and lowest amplitude within that stride
        let max = samples_averaged[start..end]
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);
        let min = samples_averaged[start..end]
            .iter()
            .cloned()
            .fold(f32::INFINITY, f32::min);
        let pixel_line = Rectangle {
            x: track_wave_background.x + pixel_column as f32,
            y: center_y - (max * TRACK_GRAPHICS_HEIGHT_HALF),
            height: (max - min) * TRACK_GRAPHICS_HEIGHT_HALF,
            width: 1.0,
        };
        pixel_line.draw(screen_config, WHITE, NO_RADIUS, out);
    }

    let open_file_button_x = (window.x + window.width) - PAD_16 - TRACK_GRAPHICS_WIDTH;
    let open_file_button_y = graphics_y + TRACK_GRAPHICS_HEIGHT + PAD_8;
    const SVG_PADDING: f32 = 4.0;

    let open_file_background = Rectangle::square(
        open_file_button_x - SVG_PADDING,
        open_file_button_y - SVG_PADDING,
        32.0 + SVG_PADDING,
    )
    .draw_style()
    .interactive(Some(mouse_state))
    .draw(screen_config, DARK_GRAY, RADIUS_4, out);

    if open_file_background.hovered && mouse_state.left_clicked {
        interaction.click = ClickResult::OpenTrackFileLocation(track.data.path.clone())
    };

    icons.push(IconDraw {
        name: "file",
        x: open_file_button_x - 2.0,
        y: open_file_button_y - 2.0,
        width: 32.0,
        height: 32.0,
        tooltip: Tooltip {
            text: Some("Open File".to_string()),
            x: (open_file_button_x),
            y: (open_file_button_y + TOOLTIP_MARGIN),
            width: DEFAULT_TOOLTIP_WIDTH,
        },
    });

    if !mouse_state.left_click_held {
        for icon in &icons {
            if icon.is_hovered(mouse_state.x, mouse_state.y) {
                tooltip = Some(icon.tooltip.clone());
            }
        }
    }

    (text_items, icons, interaction, tooltip)
}
