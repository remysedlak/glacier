use crate::color::*;
use crate::graphics::primitives::Vertex;
use crate::graphics::widgets::{window_background, window_title_bar, TextItem};
use crate::graphics::Rectangle;
use crate::graphics::{ui::*, ClickResult, ScreenConfig, PAD_16, PAD_4, PAD_8};
use crate::project::*;

// dimensions of a step
const PLAYLIST_STEP_WIDTH: f32 = 32.0;
const PLAYLIST_STEP_HEIGHT: f32 = 64.0;

// how far apart each step and track is
const PLAYLIST_STEP_GAP: f32 = 35.0;
const PLAYLIST_TRACK_GAP: f32 = 70.0;

// how far from left window edge does the timeline start
const TIMELINE_X_ORIGIN: f32 = 128.0;

// vertical line showing where we are in the song
const PLAYHEAD_WIDTH: f32 = 4.0;

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
    Vec<TextItem>,
    Vec<Vertex>,
    Vec<TextItem>,
    Vec<Vertex>,
    Vec<TextItem>,
    ClickResult,
) {
    let mut header_vertices: Vec<Vertex> = Vec::new();
    let mut header_text_items: Vec<TextItem> = Vec::new();

    let mut timeline_vertices: Vec<Vertex> = Vec::new();
    let mut timeline_text_items: Vec<TextItem> = Vec::new();

    let mut static_vertices: Vec<Vertex> = Vec::new();
    let mut static_text_items: Vec<TextItem> = Vec::new();

    // temporary constants
    let steps = 64;
    let tracks = 32;

    let mixer_background = window_background(&window);
    static_vertices.extend(mixer_background.draw(&screen_config, BLACK));

    // titlebar
    let (titlebar_verts, titlebar_texts) = window_title_bar(&window);
    static_vertices.extend(titlebar_verts.draw(&screen_config, DARK_GRAY));
    static_text_items.push(titlebar_texts);

    let mut click_result = ClickResult::None;

    static_text_items.push(TextItem {
        text: "this is my toolbar!!@!!!!".to_string(),
        x: window.x + 274.0,
        y: window.y + PAD_4,
    });

    // for each instrument loaded into a project
    for track in 0..tracks {
        // for each step in the project playlist
        for step in 0..steps {
            let group = step / 4;
            let pl_step = Rectangle {
                x: window.x + (step as f32 * PLAYLIST_STEP_GAP) + PAD_16 + (TIMELINE_X_ORIGIN) - scroll_x,
                y: window.y + (track as f32 * PLAYLIST_TRACK_GAP) + PAD_16 + PAD_32 - scroll_y,
                width: PLAYLIST_STEP_WIDTH,
                height: PLAYLIST_STEP_HEIGHT,
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

        // Track header
        let background = Rectangle {
            x: window.x + PAD_16,
            y: window.y + (track as f32 * PLAYLIST_TRACK_GAP) + PAD_32 + PAD_16 - scroll_y,
            width: TIMELINE_X_ORIGIN - PAD_4,
            height: PLAYLIST_STEP_HEIGHT,
        };
        header_vertices.extend(background.draw(&screen_config, PASCAL));
        header_text_items.push(TextItem {
            text: format!("Track {}", track).to_string(),
            x: window.x + PAD_16 + PAD_8,
            y: window.y + (track as f32 * PLAYLIST_TRACK_GAP) + PAD_32 + PAD_16 + PAD_4 - scroll_y,
        });
    }

    // Pattern rendering
    for event in events {
        if let AudioBlockType::Pattern(id) = event.block_type {
            let pl_pattern = Rectangle {
                x: window.x + (event.start_step as f32 * PLAYLIST_STEP_GAP) + PAD_16 + (TIMELINE_X_ORIGIN) - scroll_x,
                y: window.y + (event.track as f32 * PLAYLIST_TRACK_GAP) + PAD_32 + PAD_16 - scroll_y,
                width: PLAYLIST_STEP_GAP * event.length as f32,
                height: PLAYLIST_STEP_HEIGHT,
            };
            if pl_pattern.is_hovered(mouse_state.x, mouse_state.y) && mouse_state.right_clicked {
                click_result = ClickResult::DeletePlaylistPattern(event.id);
            }
            timeline_vertices.extend(pl_pattern.draw(&screen_config, pl_pattern.hover_color(mouse_state.x, mouse_state.y)));
            // titlebar text
            let label: &str = &patterns[id as usize].name;

            timeline_text_items.push(TextItem {
                text: label.to_string(),
                x: window.x + (event.start_step as f32 * PLAYLIST_STEP_GAP) + PAD_16 + (TIMELINE_X_ORIGIN) + PAD_8 - scroll_x,
                y: window.y + (event.track as f32 * PLAYLIST_TRACK_GAP) + PAD_32 + PAD_16 + PAD_4 - scroll_y,
            });
        }
    }
    let playhead = Rectangle {
        x: window.x + (current_step as f32 * PLAYLIST_STEP_GAP) + PAD_16 + (TIMELINE_X_ORIGIN) - scroll_x,
        y: window.y,
        width: PLAYHEAD_WIDTH,
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
