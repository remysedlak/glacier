use crate::colors::*;
use crate::graphics::{make_text_buffer, ScreenConfig, Vertex};
use crate::project::*;
use crate::ui::*;

pub fn draw(
    window: &MiniWindow,
    events: &[AudioBlock],
    patterns: &[PatternData],
    font_system: &mut glyphon::FontSystem,
    screen_config: &ScreenConfig,
) -> (Vec<Vertex>, Vec<(glyphon::Buffer, f32, f32)>) {
    let padding = 16.0;
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut text_items: Vec<(glyphon::Buffer, f32, f32)> = Vec::new();
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
    text_items.push((
        make_text_buffer(font_system, &window.title, 14.0, 22.0, None),
        window.x + window.width / 2.2,
        window.y - TITLEBAR_HEIGHT + 4.0,
    ));

    let steps = 32;
    let tracks = 4;

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
            vertices.extend(pl_pattern.draw(&screen_config, LIGHT_GRAY));
            // titlebar text
            let label: &str = &patterns[id as usize].name;
            text_items.push((
                make_text_buffer(font_system, label, 14.0, 22.0, Some((0, 0, 0))),
                window.x + (event.start_step as f32 * 35.0) + padding,
                window.y + (id as f32 * 70.0) + padding,
            ));
        }
    }
    (vertices, text_items)
}
