use crate::{
    app::MouseState,
    graphics::{
        color::*,
        font::TextItem,
        mini_window::{MiniWindow, WindowKind},
        primitives::{draw_rectangle, Vertex, PAD_16, PAD_4, PAD_8},
        ClickResult, ScreenConfig,
    },
};
use winit::window::CursorIcon;

/*
 * This file contains widgets.
 * Each method returns Vec<Vertex>
 */

pub const ADD_INSTRUMENT_ICON_OFFSET: f32 = 80.0;

pub const PLAY_Y_ORIGIN: f32 = 4.0;
pub const PLAY_X_ORIGIN: f32 = 90.0;

pub const TOOLBAR_Y: f32 = 42.0;
pub const TOOLBAR_THICKNESS: f32 = 0.003;
pub const TOOLBAR_MARGIN: f32 = 4.0;

pub const ICON_WIDTH: f32 = 32.0;
pub const ICON_HEIGHT: f32 = 32.0;

pub const TITLEBAR_HEIGHT: f32 = 32.0;

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
    // if either edge of a rectangle has the mouse hovered
    pub fn is_hovered_edge(&self, mouse_x: f32, mouse_y: f32) -> bool {
        ((mouse_x > self.x + self.width - PAD_8 && mouse_x < self.x + self.width + PAD_8) || (mouse_x > self.x - PAD_8 && mouse_x < self.x + PAD_8))
            && mouse_y > self.y - TITLEBAR_HEIGHT
            && mouse_y < self.y + self.height
    }
    // if the left edge of a rectangle has the mouse hovered
    pub fn is_hovered_left_edge(&self, mouse_x: f32, mouse_y: f32) -> bool {
        // on left edge within y range
        (mouse_x > self.x - PAD_4 && mouse_x < self.x + PAD_4) && mouse_y > self.y - TITLEBAR_HEIGHT && mouse_y < self.y + self.height
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
    pub fn hover_color(&self, mx: f32, my: f32, left_click_held: bool) -> (f32, f32, f32) {
        if self.is_hovered(mx, my) && !left_click_held {
            LL_GRAY
        } else {
            LIGHT_GRAY
        }
    }

    // return color for hover logic
    pub fn dark_hover_color(&self, mx: f32, my: f32, left_click_held: bool) -> (f32, f32, f32) {
        if self.is_hovered(mx, my) && !left_click_held {
            (0.05, 0.05, 0.05)
        } else {
            DARK_GRAY
        }
    }

    pub fn playlist_step_color(&self, mx: f32, my: f32, left_click_held: bool, group: u32) -> (f32, f32, f32) {
        let hovered: bool = self.is_hovered(mx, my) && !left_click_held;
        if hovered {
            if group % 2 != 0 {
                BLUE_HOVER
            } else {
                DARK_BLUE_HOVER
            }
        } else {
            if group % 2 != 0 {
                BLUE
            } else {
                DARK_BLUE
            }
        }
    }

    // return color if hovering and component is active
    pub fn active_color(&self, mx: f32, my: f32, is_active: bool, left_click_held: bool) -> (f32, f32, f32) {
        let hovered = self.is_hovered(mx, my) && !left_click_held;
        if hovered && is_active {
            ORANGE_HOVER
        } else if hovered {
            LL_GRAY
        } else if is_active {
            ORANGE
        } else {
            LIGHT_GRAY
        }
    }
    // return color for steps with velocity or the active step
    pub fn active_step_color(&self, mx: f32, my: f32, is_active: bool, has_velocity: bool, left_click_held: bool) -> (f32, f32, f32) {
        let hovered = self.is_hovered(mx, my) && !left_click_held;
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

pub fn window_title_bar(
    window: &MiniWindow,
    screen_config: &ScreenConfig,
    mouse_state: &MouseState,
) -> (Vec<Vertex>, TextItem, ClickResult, CursorIcon) {
    let mut verticies: Vec<Vertex> = Vec::new();
    let mut result = ClickResult::None;
    let mut cursor_icon = CursorIcon::Default;

    // build rectangle
    let title_bar_background = Rectangle {
        x: window.x,
        y: window.y - TITLEBAR_HEIGHT,
        width: window.width,
        height: TITLEBAR_HEIGHT,
    };
    verticies.extend(title_bar_background.draw(screen_config, DARK_GRAY));

    // add button for closing the window
    let close_window_button = Rectangle {
        x: window.x + window.width - PAD_16 - PAD_8,
        y: window.y - TITLEBAR_HEIGHT + PAD_8 + PAD_4,
        width: 15.0,
        height: 5.0,
    };
    verticies.extend(close_window_button.draw(
        screen_config,
        close_window_button.hover_color(mouse_state.x, mouse_state.y, mouse_state.left_click_held),
    ));
    if close_window_button.is_hovered(mouse_state.x, mouse_state.y) {
        cursor_icon = CursorIcon::Pointer;
        if mouse_state.left_clicked {
            result = match window.window_kind {
                WindowKind::Sequencer => ClickResult::ToggleSequencerWindow,
                WindowKind::Playlist => ClickResult::TogglePlaylistWindow,
                WindowKind::Mixer => ClickResult::ToggleMixerWindow,
                WindowKind::InstrumentDetail(usize) => ClickResult::ToggleInstrumentWindow(usize), // which instrument
            }
        }
    }
    // build text item
    let title = TextItem {
        text: window.title.to_string(),
        x: window.x + window.width / 2.2,
        y: window.y - TITLEBAR_HEIGHT + PAD_4,
        color: WHITE,
        size: 18.0,
        font: "roboto",
    };
    (verticies, title, result, cursor_icon)
}

pub fn window_background(window: &MiniWindow) -> Rectangle {
    Rectangle {
        x: window.x,
        y: window.y,
        width: window.width,
        height: window.height,
    }
}
