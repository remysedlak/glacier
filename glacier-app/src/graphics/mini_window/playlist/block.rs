//! draw one audio_block on the playlist
use winit::window::CursorIcon;

use crate::{
    app::{click::ClickResult, MouseState, ScrollOffset},
    graphics::{
        color::Color,
        font::{Font::Roboto, TextItem},
        geometry::Rectangle,
        mini_window::{
            playlist::grid::{
                GRID_X_ORIGIN, PLAYLIST_STEP_GAP, PLAYLIST_STEP_HEIGHT, PLAYLIST_TRACK_GAP,
            },
            InteractionResult, MiniWindow,
        },
        primitives::{
            ScreenConfig, Vertex, NO_RADIUS, PAD_16, PAD_4, PAD_64, PAD_8, RADIUS_8, TOP_RADIUS_8,
        },
    },
    project::{AudioBlock, AudioBlockID, AudioBlockType, PatternData, Track},
};

/// hashmap to cache min/max values of audio files for waveform drawing
pub struct WaveformCache {
    // key: audio_block_id
    // value: width, pairs
    pub entries: std::collections::HashMap<AudioBlockID, (f32, Vec<(f32, f32)>)>,
}

pub fn reduce_waveform(mono_samples: &[f32], width: usize) -> Vec<(f32, f32)> {
    if mono_samples.is_empty() {
        return Vec::new();
    }
    let num_columns = width.min(mono_samples.len()).max(1);
    let stride = (mono_samples.len() / num_columns).max(1);

    (0..num_columns)
        .filter_map(|col| {
            let start = col * stride;
            let end = (start + stride).min(mono_samples.len());
            if start >= end {
                return None;
            }
            let chunk = &mono_samples[start..end];
            let max = chunk.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let min = chunk.iter().cloned().fold(f32::INFINITY, f32::min);
            Some((min, max))
        })
        .collect()
}

// Draws a min/max reduced waveform into `rect`, scaled to fit.
/// `channels` controls mono-down; handles samples shorter than the target width.
pub fn draw_waveform(
    pairs: &[(f32, f32)],
    rect: &Rectangle,
    screen_config: &ScreenConfig,
    color: Color,
    out: &mut Vec<Vertex>,
) {
    let center_y = rect.y + rect.height / 2.0;
    let half_height = rect.height / 2.0;

    for (col, (min, max)) in pairs.iter().enumerate() {
        let pixel_line = Rectangle {
            x: rect.x + col as f32,
            y: center_y - (max * half_height),
            width: 1.0,
            height: (max - min) * half_height,
        };
        pixel_line.draw(screen_config, color, NO_RADIUS, out);
    }
}

/// build a rectangle and label for each item placed on the playlist, handle interactivity
pub fn draw_audio_block(
    tracks: &[Track],
    scroll_offset: &ScrollOffset,
    audio_block: &AudioBlock,
    window: &MiniWindow,
    mouse_state: &MouseState,
    screen_config: &ScreenConfig,
    patterns: &[PatternData],
    resizing_audio_block: Option<AudioBlockID>,
    waveform_cache: &mut WaveformCache,
    timeline_vertices: &mut Vec<Vertex>,
    timeline_text_items: &mut Vec<TextItem>,
) -> InteractionResult {
    let mut interaction = InteractionResult::default();

    let (block, label) = match audio_block.block_type {
        AudioBlockType::Pattern(id) => {
            let rect = Rectangle::new(
                window.x
                    + (audio_block.start_step as f32 * PLAYLIST_STEP_GAP)
                    + PAD_16
                    + GRID_X_ORIGIN
                    - scroll_offset.x,
                window.y + (audio_block.track_id.0 as f32 * PLAYLIST_TRACK_GAP) + PAD_64
                    - scroll_offset.y,
                PLAYLIST_STEP_GAP * audio_block.length as f32 - 2.0,
                PLAYLIST_STEP_HEIGHT,
            )
            .draw_style()
            .interactive(Some(mouse_state))
            .draw(
                screen_config,
                screen_config.palette.neutral.shade(7),
                RADIUS_8,
                timeline_vertices,
            );

            let label = patterns
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "?".to_string());
            (rect, label)
        }

        AudioBlockType::Sample(id) => {
            let rect = Rectangle::new(
                window.x
                    + (audio_block.start_step as f32 * PLAYLIST_STEP_GAP)
                    + PAD_16
                    + GRID_X_ORIGIN
                    - scroll_offset.x,
                window.y + (audio_block.track_id.0 as f32 * PLAYLIST_TRACK_GAP) + PAD_64
                    - scroll_offset.y,
                PLAYLIST_STEP_GAP * audio_block.length as f32 - 2.0,
                PLAYLIST_STEP_HEIGHT,
            )
            .draw_style()
            .interactive(Some(mouse_state))
            .draw(
                screen_config,
                screen_config.palette.neutral.shade(5),
                RADIUS_8,
                timeline_vertices,
            );

            let label = tracks
                .iter()
                .find(|t| t.data.id == id)
                .map(|t| t.data.name.clone())
                .unwrap_or_else(|| "?".to_string());

            if let Some(t) = tracks.iter().find(|t| t.data.id == id) {
                let waveform_rect = Rectangle {
                    x: rect.x,
                    y: rect.y + 24.0,
                    width: rect.width,
                    height: rect.height - 24.0,
                };

                let stale = match waveform_cache.entries.get(&audio_block.id) {
                    Some((cached_width, _)) => *cached_width != waveform_rect.width,
                    None => true,
                };
                if stale {
                    let pairs = reduce_waveform(&t.mono_samples, waveform_rect.width as usize);
                    waveform_cache
                        .entries
                        .insert(audio_block.id, (waveform_rect.width, pairs));
                }

                let (_, pairs) = waveform_cache.entries.get(&audio_block.id).unwrap();
                draw_waveform(
                    pairs,
                    &waveform_rect,
                    screen_config,
                    screen_config.palette.neutral.shade(7),
                    timeline_vertices,
                );
            }
            (rect, label)
        }
        _ => return InteractionResult::default(),
    };

    if block.x + block.width < window.x || block.x > window.x + window.width {
        return InteractionResult::default();
    }
    if block.y + block.height < window.y || block.y > window.y + window.height {
        return InteractionResult::default();
    }

    if block.hovered {
        interaction.cursor = CursorIcon::Pointer;
        if mouse_state.right_clicked {
            interaction.click = ClickResult::DeletePlaylistAudioBlock(audio_block.id);
        }
    }
    if block.right_hovered {
        interaction.cursor = CursorIcon::ColResize;
        if mouse_state.left_clicked {
            interaction.click = ClickResult::StartResizeEvent(audio_block.id);
        }
    }

    let top_line = Rectangle::new(block.x, block.y, block.width, 24.0)
        .draw_style()
        .draw(
            screen_config,
            screen_config.palette.neutral.shade(2),
            TOP_RADIUS_8,
            timeline_vertices,
        );

    timeline_text_items.push(TextItem {
        text: label,
        x: block.x + PAD_8,
        y: block.y + PAD_4,
        size: 12.0,
        font: Roboto,
        color: screen_config.palette.neutral.shade(7),
    });
    interaction
}
