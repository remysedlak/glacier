pub const ONE_MEGABYTE: u64 = 1024 * 1024;

pub const TOOLBAR_Y: f32 = 32.0;
pub const TOOLBAR_THICKNESS: f32 = 0.003;
pub const TOOLBAR_MARGIN: u32 = 4;

pub const BUTTON_X_ORIGIN: u32 = 128;
pub const BUTTON_Y_ORIGIN: u32 = 64;
pub const BUTTON_WIDTH: u32 = 24;
pub const BUTTON_HEIGHT: u32 = 64;

pub const BAR_GAP: u32 = 8;
pub const BUTTON_GAP: u32 = 32;
pub const TRACK_GAP: u32 = 72;

pub const MUTE_SQUARE_LENGTH: u32 = 12;
pub const PLAY_SQUARE_HEIGHT: u32 = ICON_HEIGHT;
pub const PLAY_SQUARE_WIDTH: u32 = 54;

pub const PLAY_Y_ORIGIN: u32 = TOOLBAR_MARGIN;
pub const PLAY_X_ORIGIN: u32 = 90;

pub const ICON_WIDTH: u32 = 32;
pub const ICON_HEIGHT: u32 = 24;

pub const LOAD_PROJECT_ICON_OFFSET: u32 = 40;
pub const ADD_INSTRUMENT_ICON_OFFSET: u32 = 80;

use crate::colors::{BLACK, DARK_GRAY, LIGHT_GRAY, LL_GRAY};
use crate::graphics::Vertex;
// this file holds my shape abstractions

#[derive(Debug)]
pub struct StepButton {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub velocity: f32,
}

// #[derive(Debug)]
// pub struct Rectangle {
//     pub x: u32,
//     pub y: u32,
//     pub width: u32,
//     pub height: u32,
// }
//
//

pub fn draw_rectangle(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    screen_width: u32,
    screen_height: u32,
    (r, g, b): (f32, f32, f32),
) -> Vec<Vertex> {
    // first normalize the coordinates to fit in decimal form.
    let ndc_x: f32 = 2.0 * (x as f32 / screen_width as f32) - 1.0;
    let ndc_y: f32 = 1.0 - (y as f32 / screen_height as f32) * 2.0;

    let ndc_width: f32 = (width as f32 / screen_width as f32) * 2.0;
    let ndc_height: f32 = (height as f32 / screen_height as f32) * 2.0;

    // next add the verticies based on these origins
    return vec![
        Vertex {
            position: [ndc_x, ndc_y, 0.0],
            color: [r, g, b],
        },
        Vertex {
            position: [ndc_x, ndc_y - ndc_height, 0.0],
            color: [r, g, b],
        },
        Vertex {
            position: [ndc_x + ndc_width, ndc_y, 0.0],
            color: [r, g, b],
        },
        Vertex {
            position: [ndc_x + ndc_width, ndc_y, 0.0],
            color: [r, g, b],
        },
        Vertex {
            position: [ndc_x, ndc_y - ndc_height, 0.0],
            color: [r, g, b],
        },
        Vertex {
            position: [ndc_x + ndc_width, ndc_y - ndc_height, 0.0],
            color: [r, g, b],
        },
    ];
}

pub fn draw_h_line(y: f32, thickness: f32, screen_height: u32) -> Vec<Vertex> {
    // first normalize the coordinates to fit in decimal form.

    let ndc_y: f32 = 1.0 - (y as f32 / screen_height as f32) * 2.0;

    return vec![
        Vertex {
            position: [-1.0, ndc_y, 0.0],
            color: [0.0, 0.0, 0.0],
        },
        Vertex {
            position: [1.0, ndc_y, 0.0],
            color: [0.0, 0.0, 0.0],
        },
        Vertex {
            position: [1.0, ndc_y - thickness, 0.0],
            color: [0.0, 0.0, 0.0],
        },
        Vertex {
            position: [-1.0, ndc_y, 0.0],
            color: [0.0, 0.0, 0.0],
        },
        Vertex {
            position: [1.0, ndc_y - thickness, 0.0],
            color: [0.0, 0.0, 0.0],
        },
        Vertex {
            position: [-1.0, ndc_y - thickness, 0.0],
            color: [0.0, 0.0, 0.0],
        },
    ];
}

pub fn draw_toolbar(
    vertices: &mut Vec<Vertex>,
    screen_x: u32,
    screen_y: u32,
    _mouse_x: f64,
    _mouse_y: f64,
) {
    // toolbar line
    for vert in draw_h_line(TOOLBAR_Y, TOOLBAR_THICKNESS, screen_y) {
        vertices.push(vert);
    }

    // bpm up button
    for vert in draw_rectangle(48, 15, 16, 6, screen_x, screen_y, LIGHT_GRAY) {
        vertices.push(vert);
    }

    // bpm down button
    for vert in draw_rectangle(48, 15 + 8, 16, 6, screen_x, screen_y, LIGHT_GRAY) {
        vertices.push(vert);
    }

    let color = if _mouse_x > PLAY_X_ORIGIN as f64
        && _mouse_x < (PLAY_X_ORIGIN + PLAY_SQUARE_WIDTH) as f64
        && _mouse_y > PLAY_Y_ORIGIN as f64
        && _mouse_y < (PLAY_Y_ORIGIN + PLAY_SQUARE_HEIGHT) as f64
    {
        LL_GRAY
    } else {
        LIGHT_GRAY
    };

    // play/pause button
    for vert in draw_rectangle(
        PLAY_X_ORIGIN,
        PLAY_Y_ORIGIN,
        PLAY_SQUARE_WIDTH,
        PLAY_SQUARE_HEIGHT,
        screen_x,
        screen_y,
        color,
    ) {
        vertices.push(vert);
    }

    let color = if _mouse_x > (screen_x - LOAD_PROJECT_ICON_OFFSET) as f64
        && _mouse_x < (screen_x - LOAD_PROJECT_ICON_OFFSET + ICON_WIDTH) as f64
        && _mouse_y > TOOLBAR_MARGIN as f64
        && _mouse_y < (TOOLBAR_MARGIN + ICON_HEIGHT) as f64
    {
        LL_GRAY
    } else {
        LIGHT_GRAY
    };

    // Load file button
    for vert in draw_rectangle(
        screen_x - LOAD_PROJECT_ICON_OFFSET,
        TOOLBAR_MARGIN,
        ICON_WIDTH,
        ICON_HEIGHT,
        screen_x,
        screen_y,
        color,
    ) {
        vertices.push(vert);
    }

    let color = if _mouse_x > (screen_x - ADD_INSTRUMENT_ICON_OFFSET) as f64
        && _mouse_x < (screen_x - ADD_INSTRUMENT_ICON_OFFSET + ICON_WIDTH) as f64
        && _mouse_y > TOOLBAR_MARGIN as f64
        && _mouse_y < (TOOLBAR_MARGIN + ICON_HEIGHT) as f64
    {
        LL_GRAY
    } else {
        LIGHT_GRAY
    };

    // add instrument button
    for vert in draw_rectangle(
        screen_x - ADD_INSTRUMENT_ICON_OFFSET,
        TOOLBAR_MARGIN,
        ICON_WIDTH,
        ICON_HEIGHT,
        screen_x,
        screen_y,
        color,
    ) {
        vertices.push(vert);
    }
}

pub fn draw_slider(
    screen_x: u32,
    screen_y: u32,
    vertices: &mut Vec<Vertex>,
    master_volume: &mut f32,
) {
    let x_coord = 64;
    let y_ceiling = 416;
    let track_height = 164;
    let track_width = 4;
    let thumb_height = 16;
    let thumb_width = 32;
    let thumb_y_coord = ((1.0 - *master_volume) * 164.0) as u32 + y_ceiling;

    // TRACK (static)
    for vert in draw_rectangle(
        x_coord + (thumb_width / 2), // the track is in the midle of the button
        y_ceiling,
        track_width,
        track_height,
        screen_x,
        screen_y,
        BLACK,
    ) {
        vertices.push(vert);
    }

    // THUMB (user input)
    for vert in draw_rectangle(
        x_coord,
        thumb_y_coord,
        thumb_width,
        thumb_height,
        screen_x,
        screen_y,
        LIGHT_GRAY,
    ) {
        vertices.push(vert);
    }
}
