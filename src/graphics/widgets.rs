use crate::color::*;
use crate::graphics::primitives::{draw_h_line, draw_rectangle, Vertex};
use crate::graphics::ui::{MiniWindow, PAD_16, PAD_4};
use crate::graphics::ScreenConfig;

pub const ADD_INSTRUMENT_ICON_OFFSET: f32 = 80.0;
pub const LOAD_PROJECT_ICON_OFFSET: f32 = 40.0;

pub const PLAY_Y_ORIGIN: f32 = 4.0;
pub const PLAY_X_ORIGIN: f32 = 90.0;

pub const TOOLBAR_Y: f32 = 32.0;
pub const TOOLBAR_THICKNESS: f32 = 0.003;
pub const TOOLBAR_MARGIN: f32 = 4.0;

pub const ICON_WIDTH: f32 = 32.0;
pub const ICON_HEIGHT: f32 = 24.0;

pub const PLAY_SQUARE_HEIGHT: f32 = ICON_HEIGHT;
pub const PLAY_SQUARE_WIDTH: f32 = 54.0;

pub const TITLEBAR_HEIGHT: f32 = 32.0;

pub struct TextItem {
    pub text: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug)]
pub struct Rectangle {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
impl Rectangle {
    // if a rectangle has the mouse hovered
    pub fn is_hovered(&self, mouse_x: f32, mouse_y: f32) -> bool {
        mouse_x > self.x && mouse_x < self.x + self.width && mouse_y > self.y && mouse_y < self.y + self.height
    }
    // draw vertices with rectangle details
    pub fn draw(&self, screen_config: &ScreenConfig, (r, g, b): (f32, f32, f32)) -> Vec<Vertex> {
        draw_rectangle(
            self.x as f32,
            self.y as f32,
            self.width as f32,
            self.height as f32,
            screen_config,
            (r, g, b),
        )
    }
    // return color for hover logic
    pub fn hover_color(&self, mx: f32, my: f32) -> (f32, f32, f32) {
        if self.is_hovered(mx, my) {
            LL_GRAY
        } else {
            LIGHT_GRAY
        }
    }

    // return color if hovering and component is active
    pub fn active_color(&self, mx: f32, my: f32, is_active: bool) -> (f32, f32, f32) {
        let hovered = self.is_hovered(mx, my);
        if hovered && is_active {
            DARK_GRAY
        } else if hovered {
            LL_GRAY
        } else if is_active {
            BLACK
        } else {
            LIGHT_GRAY
        }
    }
    // return color for steps with velocity or the active step
    pub fn active_step_color(&self, mx: f32, my: f32, is_active: bool, has_velocity: bool) -> (f32, f32, f32) {
        let hovered = self.is_hovered(mx, my);
        if is_active {
            if has_velocity {
                DARK_BLUE
            } else {
                BLUE
            }
        } else {
            if has_velocity {
                if hovered {
                    DARK_GRAY
                } else {
                    BLACK
                }
            } else {
                if hovered {
                    LL_GRAY
                } else {
                    LIGHT_GRAY
                }
            }
        }
    }
}

/// draw one slider for master panel
pub fn draw_slider(master_volume: f32, x: f32, y: f32, screen_config: &ScreenConfig) -> Vec<Vertex> {
    let x_coord = x + PAD_16;
    let y_ceiling = y + PAD_16;
    let track_height = 164.0;
    let track_width = 4.0;
    let thumb_height = 16.0;
    let thumb_width = 32.0;
    let thumb_y_coord = ((1.0 - master_volume) * 164.0) + y_ceiling;
    let mut verts: Vec<Vertex> = Vec::new();

    // TRACK (static)
    let track = Rectangle {
        x: x_coord + (thumb_width / 2.0), // the track is in the midle of the button
        y: y_ceiling,
        width: track_width,
        height: track_height,
    };
    verts.extend(track.draw(screen_config, BLACK));

    let thumb = Rectangle {
        x: x_coord,
        y: thumb_y_coord,
        width: thumb_width,
        height: thumb_height,
    };
    verts.extend(thumb.draw(screen_config, LIGHT_GRAY));
    verts
}
// draw the top toolbar
pub fn draw_toolbar(vertices: &mut Vec<Vertex>, screen_config: &ScreenConfig, _mouse_x: f32, _mouse_y: f32) {
    // toolbar line
    for vert in draw_h_line(TOOLBAR_Y, TOOLBAR_THICKNESS, screen_config) {
        vertices.push(vert);
    }

    // bpm up button
    for vert in draw_rectangle(48.0, 4.0, 32.0, 10.0, screen_config, LIGHT_GRAY) {
        vertices.push(vert);
    }

    // bpm down button
    for vert in draw_rectangle(48.0, 16.0, 32.0, 10.0, screen_config, LIGHT_GRAY) {
        vertices.push(vert);
    }

    // play or pause
    let play_button = Rectangle {
        x: PLAY_X_ORIGIN as f32,
        y: PLAY_Y_ORIGIN as f32,
        width: PLAY_SQUARE_WIDTH as f32,
        height: PLAY_SQUARE_HEIGHT as f32,
    };
    vertices.extend(play_button.draw(screen_config, play_button.hover_color(_mouse_x, _mouse_y)));

    // load a file
    let load_file_button = Rectangle {
        x: screen_config.width as f32 - LOAD_PROJECT_ICON_OFFSET,
        y: TOOLBAR_MARGIN,
        width: ICON_WIDTH,
        height: ICON_HEIGHT,
    };
    vertices.extend(load_file_button.draw(screen_config, load_file_button.hover_color(_mouse_x, _mouse_y)));

    // load an instrument
    let instrument_button = Rectangle {
        x: screen_config.width as f32 - ADD_INSTRUMENT_ICON_OFFSET,
        y: TOOLBAR_MARGIN as f32,
        width: ICON_WIDTH as f32,
        height: ICON_HEIGHT as f32,
    };
    vertices.extend(instrument_button.draw(screen_config, instrument_button.hover_color(_mouse_x, _mouse_y)));
}

pub fn window_title_bar(window: &MiniWindow) -> (Rectangle, TextItem) {
    (
        // build rectangle
        Rectangle {
            x: window.x,
            y: window.y - TITLEBAR_HEIGHT,
            width: window.width,
            height: TITLEBAR_HEIGHT,
        },
        // build text item
        TextItem {
            text: window.title.to_string(),
            x: window.x + window.width / 2.2,
            y: window.y - TITLEBAR_HEIGHT + PAD_4,
        },
    )
}

pub fn window_background(window: &MiniWindow) -> Rectangle {
    Rectangle {
        x: window.x,
        y: window.y,
        width: window.width,
        height: window.height,
    }
}
