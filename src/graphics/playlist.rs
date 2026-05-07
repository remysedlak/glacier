use crate::color::*;
use crate::graphics::{ui::*, ClickResult};
use crate::graphics::{ScreenConfig, Vertex};
use crate::project::*;

pub fn draw(
    window: &MiniWindow,
    events: &[AudioBlock],
    patterns: &[PatternData],
    mouse_state: &MouseState,

    screen_config: &ScreenConfig,
) -> (Vec<Vertex>, Vec<(String, f32, f32)>, ClickResult) {
    let padding = 16.0;
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut text_items: Vec<(String, f32, f32)> = Vec::new();
    let mixer_background = Rectangle {
        x: window.x,
        y: window.y,
        width: window.width,
        height: window.height,
    };
    vertices.extend(mixer_background.draw(&screen_config, BLACK));
    // titlebar rectangle
    let titlebar = Rectangle {
        x: window.x,
        y: window.y - TITLEBAR_HEIGHT,
        width: window.width,
        height: TITLEBAR_HEIGHT,
    };
    vertices.extend(titlebar.draw(&screen_config, DARK_GRAY));
    // titlebar text

    text_items.push((window.title.to_string(), window.x + window.width / 2.2, window.y - TITLEBAR_HEIGHT + 4.0));

    let steps = 32;
    let tracks = 4;

    let mut click_result = ClickResult::None;

    // for each instrument loaded into a project
    for track in 0..tracks {
        // for each step in the project playlist
        for step in 0..steps {
            let group = step / 4;
            let pl_step = Rectangle {
                x: window.x + (step as f32 * 35.0) + padding,
                y: window.y + (track as f32 * 70.0) + padding,
                width: 32.0,
                height: 64.0,
            };
            let color = if group % 2 != 0 { BLUE } else { DARK_BLUE };
            vertices.extend(pl_step.draw(&screen_config, color));
        }
    }
    // iterate the events to display on the playlist.
    for event in events {
        if let AudioBlockType::Pattern(id) = event.block_type {
            let pl_pattern = Rectangle {
                x: window.x + (event.start_step as f32 * 35.0) + padding,
                y: window.y + (id as f32 * 70.0) + padding,
                width: 35.0 * event.length as f32,
                height: 64.0,
            };
            if pl_pattern.is_hovered(mouse_state.x, mouse_state.y) && mouse_state.right_clicked {
                click_result = ClickResult::DeletePlaylistPattern(event.id);
            }
            vertices.extend(pl_pattern.draw(&screen_config, LIGHT_GRAY));
            // titlebar text
            let label: &str = &patterns[id as usize].name;

            text_items.push((
                label.to_string(),
                window.x + (event.start_step as f32 * 35.0) + padding,
                window.y + (id as f32 * 70.0) + padding,
            ));
        }
    }
    (vertices, text_items, click_result)
}
