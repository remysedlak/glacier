use crate::app::MouseState;
use crate::graphics::{
    color::{DARK_GRAY, MINI_WINDOW_BACKGROUND, WHITE},
    components::toolbar::TOOLTIP_MARGIN,
    icons::{IconDraw, Tooltip},
    mini_window::MiniWindow,
    primitives::{ScreenConfig, Vertex, NO_RADIUS, PAD_16, PAD_8, RADIUS_4},
    widgets::{window_background, window_title_bar, Square, TITLEBAR_HEIGHT},
    {ClickResult, Rectangle, TextItem},
};
use crate::project::Track;
use winit::window::CursorIcon;

const TRACK_GRAPHICS_WIDTH: f32 = 200.0;
const TRACK_GRAPHICS_HEIGHT: f32 = 128.0;
const TRACK_GRAPHICS_HEIGHT_HALF: f32 = 128.0 / 2.0;

pub fn draw(
    window: &MiniWindow,
    mouse_state: &MouseState,
    screen_config: &ScreenConfig,
    track: &Track,
) -> (Vec<Vertex>, Vec<TextItem>, Vec<IconDraw>, ClickResult, CursorIcon, Option<Tooltip>) {
    // setup
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut text_items: Vec<TextItem> = Vec::new();
    let mut icons: Vec<IconDraw> = Vec::new();
    let mut click_result = ClickResult::None;
    let mut cursor_icon = CursorIcon::Default;
    let mut tooltip: Option<Tooltip> = None;
    // window background
    let window_background = window_background(&window);
    vertices.extend(window_background.draw(&screen_config, MINI_WINDOW_BACKGROUND, [0.0, 16.0, 0.0, 16.0]));

    // titlebar
    let (titlebar_verts, titlebar_texts, result, cursor) =
        window_title_bar(&window, &format!("Track: {}", track.data.name), screen_config, mouse_state);
    click_result = click_result.or(result);
    if !matches!(cursor, CursorIcon::Default) {
        cursor_icon = cursor;
    }
    vertices.extend(titlebar_verts);
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
    vertices.extend(track_wave_background.draw(&screen_config, DARK_GRAY, NO_RADIUS));

    let samples_averaged: Vec<f32> = track.samples.chunks(2).map(|pair| (pair[0] + pair[1]) / 2.0).collect::<Vec<f32>>();
    let sample_stride = samples_averaged.len() / TRACK_GRAPHICS_WIDTH as usize;

    // 200 pixel columns for 200 pixel graphics
    for pixel_column in 0..199 {
        // get the first and last position of the stride
        let start = pixel_column * sample_stride as usize;
        let end = (start + sample_stride as usize).min(samples_averaged.len());
        // using  the start and end range,  find the highest and lowest amplitude within that stride
        let max = samples_averaged[start..end].iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let min = samples_averaged[start..end].iter().cloned().fold(f32::INFINITY, f32::min);
        let pixel_line = Rectangle {
            x: track_wave_background.x + pixel_column as f32,
            y: center_y - (max * TRACK_GRAPHICS_HEIGHT_HALF),
            height: (max - min) * TRACK_GRAPHICS_HEIGHT_HALF,
            width: 1.0,
        };
        vertices.extend(pixel_line.draw(screen_config, WHITE, NO_RADIUS));
    }

    let open_file_button_x = (window.x + window.width) - PAD_16 - TRACK_GRAPHICS_WIDTH;
    let open_file_button_y = graphics_y + TRACK_GRAPHICS_HEIGHT + PAD_8;
    const SVG_PADDING: f32 = 4.0;
    let open_file_background = Square {
        x: open_file_button_x - SVG_PADDING,
        y: open_file_button_y - SVG_PADDING,
        size: 32.0 + SVG_PADDING,
    };
    if open_file_background.is_hovered(mouse_state.x, mouse_state.y) {
        if mouse_state.left_clicked {
            click_result = ClickResult::OpenTrackFileLocation(track.data.path.clone())
        }
    }
    vertices.extend(open_file_background.draw(
        screen_config,
        crate::graphics::components::toolbar::icon_color(&open_file_background, mouse_state.x, mouse_state.y, mouse_state.left_click_held),
        RADIUS_4,
    ));
    icons.push(IconDraw {
        name: "file",
        x: open_file_button_x - 2.0,
        y: open_file_button_y - 2.0,
        width: 32.0,
        height: 32.0,
        tooltip: Tooltip {
            text: Some("Open File"),
            x: (open_file_button_x),
            y: (open_file_button_y + TOOLTIP_MARGIN),
        },
    });

    if !mouse_state.left_click_held {
        for icon in &icons {
            if icon.is_hovered(mouse_state.x, mouse_state.y) {
                tooltip = Some(icon.tooltip.clone());
            }
        }
    }

    (vertices, text_items, icons, click_result, cursor_icon, tooltip)
}
