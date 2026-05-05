pub const ONE_MEGABYTE: u64 = 1024 * 1024;
pub const PADDING: f32 = 16.0;
pub const TITLEBAR_HEIGHT: f32 = 32.0;
pub const TOOLBAR_Y: f32 = 32.0;
pub const TOOLBAR_THICKNESS: f32 = 0.003;
pub const TOOLBAR_MARGIN: f32 = 4.0;

pub const BUTTON_WIDTH: f32 = 18.0;
pub const BUTTON_HEIGHT: f32 = 48.0;

pub const BAR_GAP: f32 = 12.0;
pub const BUTTON_GAP: f32 = 24.0;
pub const TRACK_GAP: f32 = 72.0;

pub const KNOB_RADIUS: f32 = 13.0;

pub const MUTE_SQUARE_LENGTH: f32 = 12.0;
pub const PLAY_SQUARE_HEIGHT: f32 = ICON_HEIGHT;
pub const PLAY_SQUARE_WIDTH: f32 = 54.0;

pub const PLAY_Y_ORIGIN: f32 = TOOLBAR_MARGIN;
pub const PLAY_X_ORIGIN: f32 = 90.0;

pub const ICON_WIDTH: f32 = 32.0;
pub const ICON_HEIGHT: f32 = 24.0;

pub const LOAD_PROJECT_ICON_OFFSET: f32 = 40.0;
pub const ADD_INSTRUMENT_ICON_OFFSET: f32 = 80.0;

use std::f32::consts::PI;

use crate::colors::{BLACK, BLUE, DARK_BLUE, DARK_GRAY, LIGHT_GRAY, LL_GRAY};
use crate::graphics::Vertex;

#[derive(Debug)]
pub enum WindowKind {
    Sequencer,
    // Playlist,
    Mixer,
    // PianoRoll,
    // InstrumentDetail(usize), // which instrument
}

pub struct MiniWindow {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub title: String,
    pub is_open: bool,
    pub window_kind: WindowKind,
}
impl MiniWindow {
    /// Creates a movable new window
    pub fn new(x: f32, y: f32, width: f32, height: f32, title: &str, window_kind: WindowKind) -> Self {
        Self {
            x,
            y,
            width,
            height,
            title: title.to_string(),
            is_open: true,
            window_kind,
        }
    }
}

// for the p[la]
// enum PlaceableKind {
//     Pattern(usize),    // pattern_id
//     AudioClip(String), // path to sample
// }

// struct PlaylistEntry {
//     kind: PlaceableKind,
//     bar: usize,
//     track_row: usize, // which row in the playlist it sits on
// }

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
    pub fn draw(&self, screen_w: u32, screen_h: u32, (r, g, b): (f32, f32, f32)) -> Vec<Vertex> {
        draw_rectangle(
            self.x as f32,
            self.y as f32,
            self.width as f32,
            self.height as f32,
            screen_w,
            screen_h,
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

// draw one rectangle with one color
pub fn draw_rectangle(x: f32, y: f32, width: f32, height: f32, screen_width: u32, screen_height: u32, (r, g, b): (f32, f32, f32)) -> Vec<Vertex> {
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

// DRAW A CIRCLE USING TRIANGLE SEGMENTS
pub fn draw_circle(cx: f32, cy: f32, radius: f32, segments: u32, screen_width: u32, screen_height: u32, (r, g, b): (f32, f32, f32)) -> Vec<Vertex> {
    let mut vec: Vec<Vertex> = Vec::new();

    // first normalize the coordinates to fit in decimal form.
    let ncx: f32 = 2.0 * (cx as f32 / screen_width as f32) - 1.0;
    let ncy: f32 = 1.0 - (cy as f32 / screen_height as f32) * 2.0;
    let nrx = (radius / screen_width as f32) * 2.0;
    let nry = (radius / screen_height as f32) * 2.0;

    // draw the circle
    for k in 0..segments {
        let angle = k as f32 * (2.0 * std::f32::consts::PI / segments as f32);
        let next_angle = (k + 1) as f32 * (2.0 * std::f32::consts::PI / segments as f32);

        let x1 = ncx + nrx * angle.cos();
        let y1 = ncy + nry * angle.sin();
        let x2 = ncx + nrx * next_angle.cos();
        let y2 = ncy + nry * next_angle.sin();

        vec.push(Vertex {
            position: [ncx, ncy, 0.0],
            color: [r, g, b],
        });
        vec.push(Vertex {
            position: [x1, y1, 0.0],
            color: [r, g, b],
        });
        vec.push(Vertex {
            position: [x2, y2, 0.0],
            color: [r, g, b],
        });
    }
    vec
}

// draw a knob using circle and triangles
pub fn draw_knob(vol: f32, cx: f32, cy: f32, radius: f32, segments: u32, screen_width: u32, screen_height: u32) -> Vec<Vertex> {
    let mut vec: Vec<Vertex> = draw_circle(cx, cy, radius + 3.0, 10, screen_width, screen_height, BLACK);
    for vert in draw_circle(cx, cy, radius, segments, screen_width, screen_height, LL_GRAY) {
        vec.push(vert);
    }
    let ncx = |x: f32| 2.0 * (x as f32 / screen_width as f32) - 1.0;
    let ncy = |y: f32| 1.0 - (y as f32 / screen_height as f32) * 2.0;
    let radians = |degree: f32| (degree * PI) / 180.0;

    let angle: f32 = 210.0 - vol * 270.0; // Linear interpolation
    let x = cx + radius * radians(angle).cos();
    let y = cy - radius * radians(angle).sin();

    // draw the dial
    vec.push(Vertex {
        position: [ncx(cx), ncy(cy), 0.0], // center
        color: [0.0, 0.0, 1.0],
    });
    vec.push(Vertex {
        position: [ncx(x), ncy(y), 0.0], // hits circumfrence
        color: [0.0, 0.0, 1.0],
    });
    let perp = radians(angle + 90.0);
    vec.push(Vertex {
        position: [ncx(x) + 0.01 * perp.cos(), ncy(y) + 0.01 * perp.sin(), 0.0],
        color: [0.0, 0.0, 1.0],
    });
    vec
}

// draw a horizontal line
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

// draw the top toolbar
pub fn draw_toolbar(vertices: &mut Vec<Vertex>, screen_x: u32, screen_y: u32, _mouse_x: f32, _mouse_y: f32) {
    // toolbar line
    for vert in draw_h_line(TOOLBAR_Y, TOOLBAR_THICKNESS, screen_y) {
        vertices.push(vert);
    }

    // bpm up button
    for vert in draw_rectangle(48.0, 4.0, 32.0, 10.0, screen_x, screen_y, LIGHT_GRAY) {
        vertices.push(vert);
    }

    // bpm down button
    for vert in draw_rectangle(48.0, 16.0, 32.0, 10.0, screen_x, screen_y, LIGHT_GRAY) {
        vertices.push(vert);
    }

    // play or pause
    let play_button = Rectangle {
        x: PLAY_X_ORIGIN as f32,
        y: PLAY_Y_ORIGIN as f32,
        width: PLAY_SQUARE_WIDTH as f32,
        height: PLAY_SQUARE_HEIGHT as f32,
    };
    vertices.extend(play_button.draw(screen_x, screen_y, play_button.hover_color(_mouse_x, _mouse_y)));

    // load a file
    let load_file_button = Rectangle {
        x: screen_x as f32 - LOAD_PROJECT_ICON_OFFSET,
        y: TOOLBAR_MARGIN,
        width: ICON_WIDTH,
        height: ICON_HEIGHT,
    };
    vertices.extend(load_file_button.draw(screen_x, screen_y, load_file_button.hover_color(_mouse_x, _mouse_y)));

    // load an instrument
    let instrument_button = Rectangle {
        x: screen_x as f32 - ADD_INSTRUMENT_ICON_OFFSET,
        y: TOOLBAR_MARGIN as f32,
        width: ICON_WIDTH as f32,
        height: ICON_HEIGHT as f32,
    };
    vertices.extend(instrument_button.draw(screen_x, screen_y, instrument_button.hover_color(_mouse_x, _mouse_y)));
}

/// draw one slider for master panel
pub fn draw_slider(master_volume: &mut f32, x: f32, y: f32, screen_x: u32, screen_y: u32) -> Vec<Vertex> {
    let x_coord = x + PADDING;
    let y_ceiling = y + PADDING;
    let track_height = 164.0;
    let track_width = 4.0;
    let thumb_height = 16.0;
    let thumb_width = 32.0;
    let thumb_y_coord = ((1.0 - *master_volume) * 164.0) + y_ceiling;
    let mut verts: Vec<Vertex> = Vec::new();

    // TRACK (static)
    let track = Rectangle {
        x: x_coord + (thumb_width / 2.0), // the track is in the midle of the button
        y: y_ceiling,
        width: track_width,
        height: track_height,
    };
    verts.extend(track.draw(screen_x, screen_y, BLACK));

    let thumb = Rectangle {
        x: x_coord,
        y: thumb_y_coord,
        width: thumb_width,
        height: thumb_height,
    };
    verts.extend(thumb.draw(screen_x, screen_y, LIGHT_GRAY));
    verts
}
