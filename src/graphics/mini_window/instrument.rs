use crate::app::MouseState;
use crate::graphics::color::WHITE;
use crate::graphics::primitives::PAD_16;
use crate::graphics::{
    color::{BLACK, DARK_GRAY, MINI_WINDOW_BACKGROUND},
    mini_window::MiniWindow,
    primitives::{ScreenConfig, Vertex, NO_RADIUS, PAD_4, PAD_8, RADIUS_4},
    widgets::{window_background, window_title_bar, TITLEBAR_HEIGHT},
    {ClickResult, Rectangle, TextItem},
};
use crate::project::Instrument;
use winit::window::CursorIcon;

const INSTRUMENT_GRAPHICS_WIDTH: f32 = 200.0;
const INSTRUMENT_GRAPHICS_HEIGHT: f32 = 128.0;
const INSTRUMENT_GRAPHICS_HEIGHT_HALF: f32 = 128.0 / 2.0;

pub fn draw(
    window: &MiniWindow,
    mouse_state: &MouseState,
    screen_config: &ScreenConfig,
    track: &Instrument,
) -> (Vec<Vertex>, Vec<TextItem>, ClickResult, CursorIcon) {
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut text_items: Vec<TextItem> = Vec::new();
    let mut click_result = ClickResult::None;
    let mut cursor_icon = CursorIcon::Default;

    // window background
    let window_background = window_background(&window);
    vertices.extend(window_background.draw(&screen_config, MINI_WINDOW_BACKGROUND, [0.0, 16.0, 0.0, 16.0]));

    // titlebar
    let (titlebar_verts, titlebar_texts, result, cursor) =
        window_title_bar(&window, &format!("Instrument: {}", track.data.name), screen_config, mouse_state);
    click_result = click_result.or(result);
    if !matches!(cursor, CursorIcon::Default) {
        cursor_icon = cursor;
    }
    vertices.extend(titlebar_verts);
    text_items.push(titlebar_texts);

    // draw background of wave form for instrument
    let graphics_y = (window.y) + TITLEBAR_HEIGHT - PAD_16;
    let center_y = graphics_y + INSTRUMENT_GRAPHICS_HEIGHT / 2.0;
    let instrument_wave_background = Rectangle {
        x: (window.x + window.width) - PAD_16 - INSTRUMENT_GRAPHICS_WIDTH,
        y: graphics_y,
        width: INSTRUMENT_GRAPHICS_WIDTH,
        height: INSTRUMENT_GRAPHICS_HEIGHT,
    };
    vertices.extend(instrument_wave_background.draw(&screen_config, DARK_GRAY, NO_RADIUS));

    let samples_averaged: Vec<f32> = track.samples.chunks(2).map(|pair| (pair[0] + pair[1]) / 2.0).collect::<Vec<f32>>();
    let sample_stride = samples_averaged.len() / INSTRUMENT_GRAPHICS_WIDTH as usize;

    // 200 pixel columns for 200 pixel graphics
    for pixel_column in 0..199 {
        // get the first and last position of the stride
        let start = pixel_column * sample_stride as usize;
        let end = (start + sample_stride as usize).min(samples_averaged.len());
        // using  the start and end range,  find the highest and lowest amplitude within that stride
        let max = samples_averaged[start..end].iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let min = samples_averaged[start..end].iter().cloned().fold(f32::INFINITY, f32::min);
        let pixel_line = Rectangle {
            x: instrument_wave_background.x + pixel_column as f32,
            y: center_y - (max * INSTRUMENT_GRAPHICS_HEIGHT_HALF),
            height: (max - min) * INSTRUMENT_GRAPHICS_HEIGHT_HALF,
            width: 1.0,
        };
        vertices.extend(pixel_line.draw(screen_config, WHITE, NO_RADIUS));
    }

    (vertices, text_items, click_result, cursor_icon)
}
