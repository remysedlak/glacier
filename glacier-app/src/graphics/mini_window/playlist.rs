use crate::app::{MouseState, ScrollOffset};
use crate::graphics::{
    color::*,
    font::TextItem,
    mini_window::MiniWindow,
    primitives::*,
    widgets::{window_background, window_title_bar},
    AudioBlockType, ClickResult, PathBuf, Rectangle,
};
use crate::project::{AudioBlock, PatternData, Track};
use winit::window::CursorIcon;

const PLAYLIST_STEP_WIDTH: f32 = 32.0;
const PLAYLIST_STEP_HEIGHT: f32 = 64.0;
pub const PLAYLIST_STEP_GAP: f32 = 35.0;
const PLAYLIST_TRACK_GAP: f32 = 70.0;
const TIMELINE_X_ORIGIN: f32 = 128.0;
const PLAYHEAD_WIDTH: f32 = 4.0;

pub fn draw(
    window: &MiniWindow,
    events: &[AudioBlock],
    patterns: &[PatternData],
    tracks: &[Track],
    mouse_state: &MouseState,
    active_tray: &AudioBlockType,
    scroll_offset: &ScrollOffset,
    current_step: usize,
    resizing_event: Option<usize>,
    dragging_file: Option<&PathBuf>,
    screen_config: &ScreenConfig,
) -> (DrawRegion, DrawRegion, DrawRegion, ClickResult, CursorIcon) {
    let mut track_header_vertices: Vec<Vertex> = Vec::new();
    let mut track_header_text_items: Vec<TextItem> = Vec::new();
    let mut timeline_vertices: Vec<Vertex> = Vec::new();
    let mut timeline_text_items: Vec<TextItem> = Vec::new();
    let mut static_vertices: Vec<Vertex> = Vec::new();
    let mut static_text_items: Vec<TextItem> = Vec::new();
    let mut click_result = ClickResult::None;
    let mut cursor_icon = CursorIcon::Default;

    let step_count = 64;
    let track_count = 32;

    let playlist_background = window_background(window);
    playlist_background.draw(
        screen_config,
        MINI_WINDOW_BACKGROUND,
        BOTTOM_RADIUS_16,
        &mut static_vertices,
    );

    let (titlebar_texts, result, cursor) = window_title_bar(
        window,
        "Playlist",
        screen_config,
        mouse_state,
        &mut static_vertices,
    );
    if !matches!(cursor, CursorIcon::Default) {
        cursor_icon = cursor;
    }
    click_result = click_result.or(result);
    static_text_items.push(titlebar_texts);

    for track in 0..track_count {
        let background = Rectangle {
            x: window.x + PAD_16,
            y: window.y + (track as f32 * PLAYLIST_TRACK_GAP) + PAD_64 - scroll_offset.y,
            width: TIMELINE_X_ORIGIN - PAD_4,
            height: PLAYLIST_STEP_HEIGHT,
        };
        if background.y + background.height < window.y || background.y > window.y + window.height {
            continue;
        }
        background.draw(screen_config, PEBBLE, NO_RADIUS, &mut track_header_vertices);
        track_header_text_items.push(TextItem {
            text: format!("Track {}", track),
            x: window.x + PAD_16 + PAD_8,
            font: "roboto",
            y: window.y + (track as f32 * PLAYLIST_TRACK_GAP) + PAD_64 + PAD_4 - scroll_offset.y,
            size: 18.0,
            color: WHITE,
        });

        for step in 0..step_count {
            let group = step / 4;
            let pl_step = Rectangle {
                x: window.x + (step as f32 * PLAYLIST_STEP_GAP) + PAD_16 + TIMELINE_X_ORIGIN
                    - scroll_offset.x,
                y: window.y + (track as f32 * PLAYLIST_TRACK_GAP) + PAD_64 - scroll_offset.y,
                width: PLAYLIST_STEP_WIDTH,
                height: PLAYLIST_STEP_HEIGHT,
            };
            if pl_step.x + pl_step.width < window.x || pl_step.x > window.x + window.width {
                continue;
            }
            if pl_step.y + pl_step.height < window.y || pl_step.y > window.y + window.height {
                continue;
            }

            let hovered =
                pl_step.is_hovered(mouse_state.x, mouse_state.y) && !mouse_state.left_click_held;

            let color = if hovered {
                if dragging_file.is_some() {
                    GREEN
                } else {
                    if group % 2 != 0 {
                        BLUE_HOVER
                    } else {
                        DARK_BLUE_HOVER
                    }
                }
            } else {
                if group % 2 != 0 {
                    BLUE
                } else {
                    DARK_BLUE
                }
            };

            if pl_step.is_hovered(mouse_state.x, mouse_state.y) {
                if let Some(path) = dragging_file {
                    if mouse_state.left_released {
                        click_result = ClickResult::FSEndDragFile(path.clone(), track, step);
                    }
                } else if mouse_state.left_clicked {
                    let length = match &active_tray {
                        AudioBlockType::Pattern(id) => patterns
                            .iter()
                            .find(|p| p.id == *id)
                            .and_then(|p| p.sequences.first())
                            .map(|s| s.steps.len())
                            .unwrap_or(16),
                        _ => 1,
                    };
                    click_result = ClickResult::AddPlaylistPattern(
                        track,
                        step as u32,
                        length,
                        active_tray.clone(),
                    );
                }
            }
            pl_step.draw(screen_config, color, NO_RADIUS, &mut timeline_vertices);

            if step % 16 == 0 && track == 0 {
                timeline_text_items.push(TextItem {
                    text: format!("{group}"),
                    x: window.x + (step as f32 * PLAYLIST_STEP_GAP) + PAD_16 + TIMELINE_X_ORIGIN
                        - scroll_offset.x,
                    y: window.y + PAD_8,
                    size: 18.0,
                    font: "roboto",
                    color: WHITE,
                });
            }
        }
    }

    // event rendering
    for event in events {
        let (pl_pattern, label) = match event.block_type {
            AudioBlockType::Pattern(id) => {
                let rect = Rectangle {
                    x: window.x
                        + (event.start_step as f32 * PLAYLIST_STEP_GAP)
                        + PAD_16
                        + TIMELINE_X_ORIGIN
                        - scroll_offset.x,
                    y: window.y + (event.track as f32 * PLAYLIST_TRACK_GAP) + PAD_64
                        - scroll_offset.y,
                    width: PLAYLIST_STEP_GAP * event.length as f32 - 2.0,
                    height: PLAYLIST_STEP_HEIGHT,
                };
                let label = patterns
                    .iter()
                    .find(|p| p.id == id)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "?".to_string());
                (rect, label)
            }

            AudioBlockType::Sample(id) => {
                let rect = Rectangle {
                    x: window.x
                        + (event.start_step as f32 * PLAYLIST_STEP_GAP)
                        + PAD_16
                        + TIMELINE_X_ORIGIN
                        - scroll_offset.x,
                    y: window.y + (event.track as f32 * PLAYLIST_TRACK_GAP) + PAD_64
                        - scroll_offset.y,
                    width: PLAYLIST_STEP_GAP * event.length as f32 - 2.0,
                    height: PLAYLIST_STEP_HEIGHT,
                };
                let label = tracks
                    .iter()
                    .find(|t| t.data.id as usize == id)
                    .map(|t| t.data.name.clone())
                    .unwrap_or_else(|| "?".to_string());
                (rect, label)
            }
            _ => continue,
        };

        if pl_pattern.x + pl_pattern.width < window.x || pl_pattern.x > window.x + window.width {
            continue;
        }
        if pl_pattern.y + pl_pattern.height < window.y || pl_pattern.y > window.y + window.height {
            continue;
        }

        if pl_pattern.is_hovered(mouse_state.x, mouse_state.y) {
            cursor_icon = CursorIcon::Pointer;
            if mouse_state.right_clicked {
                click_result = ClickResult::DeletePlaylistPattern(event.id);
            }
        }
        if pl_pattern.is_hovered_right_edge(mouse_state.x, mouse_state.y) {
            cursor_icon = CursorIcon::ColResize;
            if mouse_state.left_clicked {
                click_result = ClickResult::StartResizeEvent(event.id);
            }
        }

        let pl_pattern_color =
            if pl_pattern.is_hovered(mouse_state.x, mouse_state.y) && resizing_event.is_none() {
                LIGHT_GRAY_HOVER
            } else {
                LIGHT_GRAY
            };
        pl_pattern.draw(
            screen_config,
            pl_pattern_color,
            RADIUS_8,
            &mut timeline_vertices,
        );

        timeline_text_items.push(TextItem {
            text: label,
            x: pl_pattern.x + PAD_8,
            y: pl_pattern.y + PAD_4,
            size: 18.0,
            font: "roboto",
            color: BLACK,
        });
    }

    let playhead = Rectangle {
        x: window.x + (current_step as f32 * PLAYLIST_STEP_GAP) + PAD_16 + TIMELINE_X_ORIGIN
            - scroll_offset.x,
        y: window.y + PAD_64,
        width: PLAYHEAD_WIDTH,
        height: window.height,
    };
    playhead.draw(screen_config, ORANGE, NO_RADIUS, &mut timeline_vertices);

    (
        DrawRegion {
            vertices: static_vertices,
            text_items: static_text_items,
        },
        DrawRegion {
            vertices: timeline_vertices,
            text_items: timeline_text_items,
        },
        DrawRegion {
            vertices: track_header_vertices,
            text_items: track_header_text_items,
        },
        click_result,
        cursor_icon,
    )
}
