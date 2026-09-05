use crate::app::{click::ClickResult, MouseState, ScrollOffset};
use crate::graphics::mini_window::InteractionResult;
use crate::graphics::{
    color::*, font::TextItem, mini_window::MiniWindow, primitives::*, AudioBlockType, PathBuf,
};
use crate::project::{AudioBlock, PatternData, Track};
use winit::window::CursorIcon;

pub mod block;
pub mod grid;
pub mod playhead;
mod toolbar;

/// Draw the playlist Mini Window. This is where the user composes the entire song and project. Instruments can be placed here from the track tray, and patterns from the pattern track.
pub fn draw(
    window: &MiniWindow,
    audio_blocks: &[AudioBlock],
    patterns: &[PatternData],
    tracks: &[Track],
    mouse_state: &MouseState,
    active_tray: &AudioBlockType,
    scroll_offset: &ScrollOffset,
    playhead_beat: f32, // was: current_step: usize
    resizing_audio_block: Option<usize>,
    dragging_file: Option<&PathBuf>,
    screen_config: &ScreenConfig,
) -> (DrawRegion, DrawRegion, DrawRegion, InteractionResult) {
    // setup
    let mut static_vertices: Vec<Vertex> = Vec::new();
    let mut static_text_items: Vec<TextItem> = Vec::new();
    let mut interaction = InteractionResult::default();
    

    // lazy implementation - TODO: add dynamic track count and step count for projects
    let step_count = 64;
    let track_count = 32;

    let playlist_background = window.background();
    playlist_background.draw(
        screen_config,
        MINI_WINDOW_BACKGROUND,
        [0.0, 16.0, 0.0, 16.0],
        &mut static_vertices,
    );

    let (titlebar_texts, titlebar_interaction) =
        window.title_bar("Playlist", screen_config, mouse_state, &mut static_vertices);
    interaction = interaction.or(titlebar_interaction);
    static_text_items.push(titlebar_texts);

    // where u place sounds
    let (
        track_header_vertices,
        track_header_text_items,
        mut timeline_vertices,
        mut timeline_text_items,
        result,
    ) = grid::draw(
        window,
        track_count,
        step_count,
        screen_config,
        mouse_state,
        scroll_offset,
        dragging_file,
        active_tray,
        patterns,
    );

    // render

    for audio_block in audio_blocks {
        let block_interaction = block::draw_audio_block(
            tracks,
            scroll_offset,
            audio_block,
            window,
            mouse_state,
            screen_config,
            patterns,
            resizing_audio_block,
            &mut timeline_vertices,
            &mut timeline_text_items,
        );
        interaction = interaction.or(block_interaction)
    }

    // draw playhead at the current beat
    playhead::draw(playhead_beat, window, scroll_offset).draw(
        screen_config,
        ORANGE,
        NO_RADIUS,
        &mut timeline_vertices,
    );

    // return draw regions and mouse state
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
        interaction,
    )
}
