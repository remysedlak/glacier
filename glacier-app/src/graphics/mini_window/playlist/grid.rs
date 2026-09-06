use std::path::PathBuf;

use crate::{
    app::{click::ClickResult, MouseState, ScrollOffset},
    graphics::{
        color::{BLUE, DARK_BLUE, GREEN, SURFACE, WHITE},
        font::{TextItem, ROBOTO},
        geometry::Rectangle,
        icons::IconDraw,
        mini_window::{
            playlist::toolbar::{self, PlaylistTool},
            InteractionResult, MiniWindow,
        },
        primitives::{ScreenConfig, Vertex, NO_RADIUS, PAD_16, PAD_4, PAD_64, PAD_8},
    },
    project::{AudioBlockType, PatternData, TrackID},
};

pub const PLAYLIST_STEP_WIDTH: f32 = 32.0;
pub const PLAYLIST_STEP_HEIGHT: f32 = 64.0;
pub const PLAYLIST_STEP_GAP: f32 = 35.0;
pub const PLAYLIST_TRACK_GAP: f32 = 70.0;
pub const GRID_X_ORIGIN: f32 = 128.0;
pub const PLAYHEAD_WIDTH: f32 = 4.0;

pub fn draw(
    window: &MiniWindow,
    track_count: usize,
    step_count: usize,
    screen_config: &ScreenConfig,
    mouse_state: &MouseState,
    scroll_offset: &ScrollOffset,
    dragging_file: Option<&PathBuf>,
    active_tray: &AudioBlockType,
    patterns: &[PatternData],
    playlist_tool: &PlaylistTool,
) -> (
    Vec<Vertex>,
    Vec<TextItem>,
    Vec<IconDraw>,
    Vec<Vertex>,
    Vec<TextItem>,
    Vec<Vertex>,
    Vec<TextItem>,
    InteractionResult,
) {
    let mut track_header_vertices: Vec<Vertex> = Vec::new();
    let mut track_header_text_items: Vec<TextItem> = Vec::new();

    let mut grid_vertices: Vec<Vertex> = Vec::new();
    let grid_text_items: Vec<TextItem> = Vec::new();

    let mut interaction = InteractionResult::default();

    let (toolbar_vertices, toolbar_text_items, toolbar_icons, toolbar_interaction) =
        toolbar::draw(window, screen_config, mouse_state, playlist_tool);

    interaction = interaction.or(toolbar_interaction);

    // for each ui track
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
            SURFACE,
            NO_RADIUS,
            &mut track_header_vertices,
        );
        track_header_text_items.push(TextItem {
            text: format!("Track {}", track),
            x: window.x + PAD_16 + PAD_8,
            font: ROBOTO,
            y: window.y + (track as f32 * PLAYLIST_TRACK_GAP) + PAD_64 + PAD_4 - scroll_offset.y,
            size: 18.0,
            color: WHITE,
        });

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

            let base = if group % 2 != 0 { BLUE } else { DARK_BLUE };
            let color = if dragging_file.is_some() && hovered {
                GREEN
            } else if hovered {
                base.hovered()
            } else {
                base
            };

            if pl_step.is_hovered(mouse_state.x, mouse_state.y) {
                if let Some(path) = dragging_file {
                    if mouse_state.left_released {
                        interaction.click =
                            ClickResult::FSEndDragFile(path.clone(), TrackID(track as u32), step);
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
                    interaction.click = ClickResult::AddPlaylistAudioBlock(
                        TrackID(track as u32),
                        step as u32,
                        length.try_into().unwrap(),
                        active_tray.clone(),
                    );
                }
            }
            pl_step.draw(screen_config, color, NO_RADIUS, &mut grid_vertices);
        }
    }
    (
        toolbar_vertices,
        toolbar_text_items,
        toolbar_icons,
        track_header_vertices,
        track_header_text_items,
        grid_vertices,
        grid_text_items,
        interaction,
    )
}
