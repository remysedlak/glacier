//! draw one audio_block on the playlist
use winit::window::CursorIcon;

use crate::{
    app::{click::ClickResult, MouseState, ScrollOffset},
    graphics::{
        color::{BLACK, LIGHT_GRAY},
        font::{TextItem, ROBOTO},
        geometry::Rectangle,
        mini_window::{
            playlist::grid::{
                PLAYLIST_STEP_GAP, PLAYLIST_STEP_HEIGHT, PLAYLIST_TRACK_GAP, TIMELINE_X_ORIGIN,
            },
            MiniWindow,
        },
        primitives::{ScreenConfig, Vertex, PAD_16, PAD_4, PAD_64, PAD_8, RADIUS_8},
    },
    project::{AudioBlock, AudioBlockType, PatternData, Track},
};

/// build a rectangle and label for each item placed on the playlist, handle interactivity
pub fn draw_audio_block(
    tracks: &[Track],
    scroll_offset: &ScrollOffset,
    audio_block: &AudioBlock,
    window: &MiniWindow,
    mouse_state: &MouseState,
    screen_config: &ScreenConfig,
    patterns: &[PatternData],
    resizing_audio_block: Option<usize>,
    timeline_vertices: &mut Vec<Vertex>,
    timeline_text_items: &mut Vec<TextItem>,
) -> (ClickResult, CursorIcon) {
    let mut click_result = ClickResult::None;
    let mut cursor_icon = CursorIcon::Default;

    let (block, label) = match audio_block.block_type {
        AudioBlockType::Pattern(id) => {
            let rect = Rectangle {
                x: window.x
                    + (audio_block.start_step as f32 * PLAYLIST_STEP_GAP)
                    + PAD_16
                    + TIMELINE_X_ORIGIN
                    - scroll_offset.x,
                y: window.y + (audio_block.track as f32 * PLAYLIST_TRACK_GAP) + PAD_64
                    - scroll_offset.y,
                width: PLAYLIST_STEP_GAP * audio_block.length as f32 - 2.0,
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
                    + (audio_block.start_step as f32 * PLAYLIST_STEP_GAP)
                    + PAD_16
                    + TIMELINE_X_ORIGIN
                    - scroll_offset.x,
                y: window.y + (audio_block.track as f32 * PLAYLIST_TRACK_GAP) + PAD_64
                    - scroll_offset.y,
                width: PLAYLIST_STEP_GAP * audio_block.length as f32 - 2.0,
                height: PLAYLIST_STEP_HEIGHT,
            };
            let label = tracks
                .iter()
                .find(|t| t.data.id as usize == id)
                .map(|t| t.data.name.clone())
                .unwrap_or_else(|| "?".to_string());
            (rect, label)
        }
        _ => return (ClickResult::None, CursorIcon::Default),
    };

    if block.x + block.width < window.x || block.x > window.x + window.width {
        return (ClickResult::None, CursorIcon::Default);
    }
    if block.y + block.height < window.y || block.y > window.y + window.height {
        return (ClickResult::None, CursorIcon::Default);
    }

    if block.is_hovered(mouse_state.x, mouse_state.y) {
        cursor_icon = CursorIcon::Pointer;
        if mouse_state.right_clicked {
            click_result = ClickResult::DeletePlaylistAudioBlock(audio_block.id);
        }
    }
    if block.is_hovered_right_edge(mouse_state.x, mouse_state.y) {
        cursor_icon = CursorIcon::ColResize;
        if mouse_state.left_clicked {
            click_result = ClickResult::StartResizeEvent(audio_block.id);
        }
    }

    let block_color =
        if block.is_hovered(mouse_state.x, mouse_state.y) && resizing_audio_block.is_none() {
            LIGHT_GRAY.hovered()
        } else {
            LIGHT_GRAY
        };
    block.draw(screen_config, block_color, RADIUS_8, timeline_vertices);

    timeline_text_items.push(TextItem {
        text: label,
        x: block.x + PAD_8,
        y: block.y + PAD_4,
        size: 18.0,
        font: ROBOTO,
        color: BLACK,
    });
    (click_result, cursor_icon)
}
