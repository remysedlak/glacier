use crate::color::*;
use crate::graphics::{ui::*, ClickResult};
use crate::graphics::{ScreenConfig, Vertex};
use crate::project::*;

pub fn draw(
    window: &MiniWindow,
    events: &[AudioBlock],
    patterns: &[PatternData],
    mouse_state: &MouseState,
    active_pattern_id: usize,
    scroll_x: f32,
    scroll_y: f32,
    current_step: usize,
    screen_config: &ScreenConfig,
) -> (
    Vec<Vertex>,
    Vec<(String, f32, f32)>,
    Vec<Vertex>,
    Vec<(String, f32, f32)>,
    Vec<Vertex>,
    Vec<(String, f32, f32)>,
    ClickResult,
) {
    let padding = 16.0;
    let mini_pad = (padding / 2.0, padding / 4.0);

    let mut header_vertices: Vec<Vertex> = Vec::new();
    let mut header_text_items: Vec<(String, f32, f32)> = Vec::new();

    let mut timeline_vertices: Vec<Vertex> = Vec::new();
    let mut timeline_text_items: Vec<(String, f32, f32)> = Vec::new();

    let mut static_vertices: Vec<Vertex> = Vec::new();
    let mut static_text_items: Vec<(String, f32, f32)> = Vec::new();

    let mixer_background = Rectangle {
        x: window.x,
        y: window.y,
        width: window.width,
        height: window.height,
    };
    static_vertices.extend(mixer_background.draw(&screen_config, BLACK));
    // titlebar rectangle
    let titlebar = Rectangle {
        x: window.x,
        y: window.y - TITLEBAR_HEIGHT,
        width: window.width,
        height: TITLEBAR_HEIGHT,
    };
    static_vertices.extend(titlebar.draw(&screen_config, DARK_GRAY));
    // titlebar text

    static_text_items.push((
        window.title.to_string(),
        window.x + (window.width / 2.2),
        window.y - TITLEBAR_HEIGHT + 4.0,
    ));

    let steps = 64;
    let tracks = 32;

    let mut click_result = ClickResult::None;
    let step_padding = 32.0;

    static_text_items.push(("this is my toolbar!!@!!!!".to_string(), window.x + 274.0, window.y + 8.0));

    // for each instrument loaded into a project
    for track in 0..tracks {
        // for each step in the project playlist
        for step in 0..steps {
            let group = step / 4;
            let pl_step = Rectangle {
                x: window.x + (step as f32 * 35.0) + padding + (step_padding * 4.0) - scroll_x,
                y: window.y + (track as f32 * 70.0) + padding + step_padding - scroll_y,
                width: 32.0,
                height: 64.0,
            };

            if pl_step.is_hovered(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
                // add a new sequence with the active pattern at this step
                click_result = ClickResult::AddPlaylistPattern(
                    track,
                    step,
                    patterns[active_pattern_id].sequences[0].steps.len(),
                    AudioBlockType::Pattern(active_pattern_id),
                );
            }
            let color = if group % 2 != 0 { BLUE } else { DARK_BLUE };
            timeline_vertices.extend(pl_step.draw(&screen_config, color));
        }

        let background = Rectangle {
            x: window.x + 16.0,
            y: window.y + (track as f32 * 70.0) + padding + step_padding - scroll_y,
            width: 124.0,
            height: 64.0,
        };
        header_vertices.extend(background.draw(&screen_config, PASCAL));
        let label = format!("Track {}", track);
        header_text_items.push((
            label.to_string(),
            window.x + 16.0 + mini_pad.0,
            window.y + (track as f32 * 70.0) + padding + step_padding + mini_pad.1 - scroll_y,
        ));
    }

    for event in events {
        if let AudioBlockType::Pattern(id) = event.block_type {
            let pl_pattern = Rectangle {
                x: window.x + (event.start_step as f32 * 35.0) + padding + (step_padding * 4.0) - scroll_x,
                y: window.y + (event.track as f32 * 70.0) + padding + step_padding - scroll_y,
                width: 32.0 * event.length as f32,
                height: 64.0,
            };
            if pl_pattern.is_hovered(mouse_state.x, mouse_state.y) && mouse_state.right_clicked {
                click_result = ClickResult::DeletePlaylistPattern(event.id);
            }
            timeline_vertices.extend(pl_pattern.draw(&screen_config, pl_pattern.hover_color(mouse_state.x, mouse_state.y)));
            // titlebar text
            let label: &str = &patterns[id as usize].name;

            timeline_text_items.push((
                label.to_string(),
                window.x + (event.start_step as f32 * 35.0) + padding + (step_padding * 4.0) - scroll_x + mini_pad.0,
                window.y + (event.track as f32 * 70.0) + padding + step_padding - scroll_y + mini_pad.1,
            ));
        }
    }
    let playhead = Rectangle {
        x: window.x + (current_step as f32 * 35.0) + padding + (step_padding * 4.0) - scroll_x,
        y: window.y,
        width: 4.0,
        height: window.height,
    };
    timeline_vertices.extend(playhead.draw(&screen_config, ORANGE));
    (
        static_vertices,
        static_text_items,
        timeline_vertices,
        timeline_text_items,
        header_vertices,
        header_text_items,
        click_result,
    )
}
