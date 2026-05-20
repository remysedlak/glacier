use crate::app::MouseState;
use crate::graphics::color::{BLUE_HOVER, DARK_BLUE_HOVER};
use crate::graphics::{
    color::{BLUE, DARK_BLUE, DARK_GRAY, ORANGE, PEBBLE, WHITE},
    mini_window::{MiniWindow, MINI_WINDOW_BACKGROUND},
    primitives::{ScreenConfig, Vertex, PAD_16, PAD_32, PAD_4, PAD_64, PAD_8},
    widgets::{window_background, window_title_bar, TextItem},
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
    let mut header_vertices: Vec<Vertex> = Vec::new();
    let mut header_text_items: Vec<TextItem> = Vec::new();

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
    static_vertices.extend(playlist_background.draw(&screen_config, MINI_WINDOW_BACKGROUND));

    // titlebar
    let (titlebar_verts, titlebar_texts) = window_title_bar(&window);
    static_vertices.extend(titlebar_verts.draw(&screen_config, DARK_GRAY));
    static_text_items.push(titlebar_texts);

    // let buttons = 8;
    // for i in 0..buttons {
    //     let toolbar_button_background = Rectangle {
    //         x: window.x + PAD_16 + (i as f32 * 64.0) - 2.0,
    //         y: window.y + PAD_8 - 2.0,
    //         width: 32.0 + 4.0,
    //         height: 24.0 + 4.0,
    //     };
    //     let toolbar_button = Rectangle {
    //         x: window.x + PAD_16 + (i as f32 * 64.0),
    //         y: window.y + PAD_8,
    //         width: 32.0,
    //         height: 24.0,
    //     };
    //     static_vertices.extend(toolbar_button_background.draw(&screen_config, ORANGE));
    //     static_vertices.extend(toolbar_button.draw(&screen_config, LL_GRAY));
    // }

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

            // add a new event with the active pattern at this step
            let color = pl_step.playlist_step_color(mouse_state.x, mouse_state.y, mouse_state.left_click_held, group);
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

            timeline_vertices.extend(pl_step.draw(&screen_config, color));

            // every 4 measure display the measure (zoom feature later shows more measure labels)
            if step % 16 == 0 {
                static_text_items.push(TextItem {
                    text: format!("{group}").to_string(),
                    x: window.x + (step as f32 * PLAYLIST_STEP_GAP) + PAD_16 + (TIMELINE_X_ORIGIN) - scroll_x,
                    y: window.y + (track as f32 * PLAYLIST_TRACK_GAP) + PAD_32,
                    size: 18.0,
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
        header_vertices.extend(background.draw(&screen_config, PEBBLE));
        header_text_items.push(TextItem {
            text: format!("Track {}", track).to_string(),
            x: window.x + PAD_16 + PAD_8,
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
            timeline_vertices.extend(pl_pattern.draw(
                &screen_config,
                pl_pattern.hover_color(mouse_state.x, mouse_state.y, mouse_state.left_click_held),
            ));
            // titlebar text
            let label: &str = &patterns[id as usize].name;

            timeline_text_items.push(TextItem {
                text: label.to_string(),
                x: window.x + (event.start_step as f32 * PLAYLIST_STEP_GAP) + PAD_16 + (TIMELINE_X_ORIGIN) + PAD_8 - scroll_x,
                y: window.y + (event.track as f32 * PLAYLIST_TRACK_GAP) + PAD_64 + PAD_4 - scroll_y,
                size: 18.0,
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
    timeline_vertices.extend(playhead.draw(&screen_config, ORANGE));
    (
        static_vertices,
        static_text_items,
        timeline_vertices,
        timeline_text_items,
        header_vertices,
        header_text_items,
        click_result,
        cursor_icon,
    )
}
