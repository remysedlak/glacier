use crate::app::MouseState;
use crate::graphics::color::{BLUE, BLUE_HOVER, DARK_BLUE, DARK_BLUE_HOVER, LIGHT_GRAY, LL_GRAY};
use crate::graphics::primitives::{BOTTOM_RADIUS, NO_RADIUS};
use crate::graphics::{
    color::{ORANGE, PEBBLE, WHITE},
    font::TextItem,
    mini_window::{MiniWindow, MINI_WINDOW_BACKGROUND},
    primitives::{ScreenConfig, Vertex, PAD_16, PAD_32, PAD_4, PAD_64, PAD_8},
    widgets::{window_background, window_title_bar},
    AudioBlockType, ClickResult, Rectangle,
};
use crate::project::{AudioBlock, PatternData};
use winit::window::CursorIcon;

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
    CursorIcon,
) {
    let mut track_header_vertices: Vec<Vertex> = Vec::new();
    let mut track_header_text_items: Vec<TextItem> = Vec::new();

    let mut timeline_vertices: Vec<Vertex> = Vec::new();
    let mut timeline_text_items: Vec<TextItem> = Vec::new();

    let mut static_vertices: Vec<Vertex> = Vec::new();
    let mut static_text_items: Vec<TextItem> = Vec::new();

    let mut click_result = ClickResult::None;
    let mut cursor_icon = CursorIcon::Default;

    // temporary constants
    let steps = 64;
    let tracks = 32;

    let playlist_background = window_background(&window);
    static_vertices.extend(playlist_background.draw(&screen_config, MINI_WINDOW_BACKGROUND, BOTTOM_RADIUS));

    // titlebar
    let (titlebar_verts, titlebar_texts, result, cursor) = window_title_bar(&window, screen_config, mouse_state);
    if !matches!(cursor, CursorIcon::Default) {
        cursor_icon = cursor;
    }
    click_result = click_result.or(result);
    static_vertices.extend(titlebar_verts);
    static_text_items.push(titlebar_texts);

    // for each instrument loaded into a project
    for track in 0..tracks {
        // for each step in the project playlist
        for step in 0..steps {
            let group = step / 4;
            let pl_step = Rectangle {
                x: window.x + (step as f32 * PLAYLIST_STEP_GAP) + PAD_16 + (TIMELINE_X_ORIGIN) - scroll_x,
                y: window.y + (track as f32 * PLAYLIST_TRACK_GAP) + PAD_64 - scroll_y,
                width: PLAYLIST_STEP_WIDTH,
                height: PLAYLIST_STEP_HEIGHT,
            };
            if pl_step.x + pl_step.width < window.x || pl_step.x > window.x + window.width {
                continue;
            }
            if pl_step.y + pl_step.height < window.y || pl_step.y > window.y + window.height {
                continue;
            }

            // add a new event with the active pattern at this step
            let hovered: bool = pl_step.is_hovered(mouse_state.x, mouse_state.y) && !mouse_state.left_click_held;
            let color = if hovered {
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
            };

            if pl_step.is_hovered(mouse_state.x, mouse_state.y) {
                if mouse_state.left_clicked {
                    click_result = ClickResult::AddPlaylistPattern(
                        track,
                        step,
                        patterns[active_pattern_id].sequences[0].steps.len(),
                        AudioBlockType::Pattern(active_pattern_id),
                    );
                }
            }

            timeline_vertices.extend(pl_step.draw(&screen_config, color, NO_RADIUS));

            // every 4 measure display the measure (zoom feature later shows more measure labels)
            if step % 16 == 0 {
                static_text_items.push(TextItem {
                    text: format!("{group}").to_string(),
                    x: window.x + (step as f32 * PLAYLIST_STEP_GAP) + PAD_16 + (TIMELINE_X_ORIGIN) - scroll_x,
                    y: window.y + (track as f32 * PLAYLIST_TRACK_GAP) + PAD_32,
                    size: 18.0,
                    font: "roboto",
                    color: WHITE,
                });
            }
        }

        // Track header
        let background = Rectangle {
            x: window.x + PAD_16,
            y: window.y + (track as f32 * PLAYLIST_TRACK_GAP) + PAD_64 - scroll_y,
            width: TIMELINE_X_ORIGIN - PAD_4,
            height: PLAYLIST_STEP_HEIGHT,
        };
        if background.y + background.height < window.y || background.y > window.y + window.height {
            continue;
        }
        track_header_vertices.extend(background.draw(&screen_config, PEBBLE, NO_RADIUS));
        track_header_text_items.push(TextItem {
            text: format!("Track {}", track).to_string(),
            x: window.x + PAD_16 + PAD_8,
            font: "roboto",
            y: window.y + (track as f32 * PLAYLIST_TRACK_GAP) + PAD_64 + PAD_4 - scroll_y,
            size: 18.0,
            color: WHITE,
        });
    }

    // Pattern rendering
    for event in events {
        if let AudioBlockType::Pattern(id) = event.block_type {
            let pl_pattern = Rectangle {
                x: window.x + (event.start_step as f32 * PLAYLIST_STEP_GAP) + PAD_16 + (TIMELINE_X_ORIGIN) - scroll_x,
                y: window.y + (event.track as f32 * PLAYLIST_TRACK_GAP) + PAD_64 - scroll_y,
                width: PLAYLIST_STEP_GAP * event.length as f32,
                height: PLAYLIST_STEP_HEIGHT,
            };

            if pl_pattern.x + pl_pattern.width < window.x || pl_pattern.x > window.x + window.width {
                continue;
            }
            if pl_pattern.y + pl_pattern.height < window.y || pl_pattern.y > window.y + window.height {
                continue;
            }

            // delete a placed pattern
            if pl_pattern.is_hovered(mouse_state.x, mouse_state.y) {
                cursor_icon = CursorIcon::Pointer;
                if mouse_state.right_clicked {
                    click_result = ClickResult::DeletePlaylistPattern(event.id);
                }
            }
            if pl_pattern.is_hovered_edge(mouse_state.x, mouse_state.y) {
                cursor_icon = CursorIcon::ColResize
            }
            let pl_pattern_color = if pl_pattern.is_hovered(mouse_state.x, mouse_state.y) {
                LL_GRAY
            } else {
                LIGHT_GRAY
            };
            timeline_vertices.extend(pl_pattern.draw(&screen_config, pl_pattern_color, NO_RADIUS));
            // titlebar text
            let label: &str = &patterns[id as usize].name;

            timeline_text_items.push(TextItem {
                text: label.to_string(),
                x: window.x + (event.start_step as f32 * PLAYLIST_STEP_GAP) + PAD_16 + (TIMELINE_X_ORIGIN) + PAD_8 - scroll_x,
                y: window.y + (event.track as f32 * PLAYLIST_TRACK_GAP) + PAD_64 + PAD_4 - scroll_y,
                size: 18.0,
                font: "roboto",
                color: WHITE,
            });
        }
    }
    let playhead = Rectangle {
        x: window.x + (current_step as f32 * PLAYLIST_STEP_GAP) + PAD_16 + (TIMELINE_X_ORIGIN) - scroll_x,
        y: window.y + PAD_64,
        width: PLAYHEAD_WIDTH,
        height: window.height,
    };
    timeline_vertices.extend(playhead.draw(&screen_config, ORANGE, NO_RADIUS));
    (
        static_vertices,
        static_text_items,
        timeline_vertices,
        timeline_text_items,
        track_header_vertices,
        track_header_text_items,
        click_result,
        cursor_icon,
    )
}
