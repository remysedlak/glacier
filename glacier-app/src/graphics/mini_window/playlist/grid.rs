use std::path::PathBuf;

use crate::{
    app::{click::ClickResult, MouseState, ScrollOffset},
    graphics::{
        font::{Font::Roboto, TextItem},
        geometry::Rectangle,
        mini_window::{playlist::PlaylistAction, InteractionResult, MiniWindow},
        primitives::{ScreenConfig, NO_RADIUS, PAD_16, PAD_4, PAD_64, PAD_8},
        FrameDrawContext,
    },
    project::{AudioBlockType, PatternData},
};

pub const PLAYLIST_STEP_WIDTH: f32 = 32.0;
pub const PLAYLIST_STEP_HEIGHT: f32 = 64.0;
pub const PLAYLIST_STEP_GAP: f32 = 35.0;
pub const PLAYLIST_TRACK_GAP: f32 = 70.0;
pub const GRID_X_ORIGIN: f32 = 128.0;
pub const PLAYHEAD_WIDTH: f32 = 4.0;

pub fn draw_track_headers(
    window: &MiniWindow,
    track_count: usize,
    screen_config: &ScreenConfig,
    scroll_offset: &ScrollOffset,
    frame: &mut FrameDrawContext<'_>,
) {
    for track in 0..track_count {
        let background = Rectangle {
            x: window.x + PAD_16,
            y: window.y + (track as f32 * PLAYLIST_TRACK_GAP) + PAD_64 - scroll_offset.y,
            width: GRID_X_ORIGIN - PAD_4,
            height: PLAYLIST_STEP_HEIGHT,
        };
        if background.y + background.height < window.y || background.y > window.y + window.height {
            continue;
        }
        background.draw(
            screen_config,
            screen_config.palette.neutral.shade(2),
            NO_RADIUS,
            frame,
        );
        frame.draw_texts(&[TextItem {
            text: format!("Track {}", track),
            x: window.x + PAD_16 + PAD_8,
            font: Roboto,
            y: window.y + (track as f32 * PLAYLIST_TRACK_GAP) + PAD_64 + PAD_4 - scroll_offset.y,
            size: 18.0,
            color: screen_config.palette.neutral.shade(7),
        }]);
    }
}

pub fn draw_grid(
    window: &MiniWindow,
    track_count: usize,
    step_count: usize,
    screen_config: &ScreenConfig,
    mouse_state: &MouseState,
    scroll_offset: &ScrollOffset,
    dragging_file: Option<&PathBuf>,
    playlist_draw: PlaylistAction,
    patterns: &[PatternData],
    frame: &mut FrameDrawContext<'_>,
) -> InteractionResult {
    let mut interaction = InteractionResult::default();
    for track in 0..track_count {
        for step in 0..step_count {
            let group = step / 4;
            let pl_step = Rectangle {
                x: window.x + (step as f32 * PLAYLIST_STEP_GAP) + PAD_16 + GRID_X_ORIGIN
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

            let base = if group % 2 != 0 {
                screen_config.palette.primary.shade(4)
            } else {
                screen_config.palette.primary.shade(3)
            };
            let color = if dragging_file.is_some() && hovered {
                screen_config.palette.secondary.shade(5)
            } else {
                base
            };
            // dbg!(
            //     screen_config.palette.primary.shade(4),
            //     screen_config.palette.primary.shade(3)
            // );

            // panic!("oops");

            if pl_step.is_hovered(mouse_state.x, mouse_state.y) {
                if let Some(path) = dragging_file {
                    if mouse_state.left_released {
                        interaction.click =
                            ClickResult::FSEndDragFile(path.clone(), track as u32, step);
                    }
                } else if mouse_state.left_clicked {
                    match playlist_draw {
                        PlaylistAction::PlaceCopy(source_id) => {
                            interaction.click = ClickResult::DuplicatePlaylistAudioBlock(
                                source_id,
                                track as u32,
                                step as u32,
                            );
                        }
                        PlaylistAction::PlaceFresh(block_type) => {
                            let length = match block_type {
                                AudioBlockType::Pattern(id) => patterns
                                    .iter()
                                    .find(|p| p.id == id)
                                    .and_then(|p| p.sequences.first())
                                    .map(|s| s.steps.len())
                                    .unwrap_or(16),
                                _ => 1,
                            };
                            interaction.click = ClickResult::AddPlaylistAudioBlock(
                                track as u32,
                                step as u32,
                                length.try_into().unwrap(),
                                block_type,
                            );
                        }
                        PlaylistAction::None => {}
                    }
                }
            }

            pl_step.draw(screen_config, color, NO_RADIUS, frame);
        }
    }
    interaction
}
