use crate::app::{MouseState, ScrollOffset, click::ClickResult};
use crate::graphics::PlaylistAction;
use crate::graphics::{
    FrameDrawContext, PathBuf,
    geometry::checkbox,
    mini_window::{
        InteractionResult, MiniWindow, TITLEBAR_HEIGHT,
        playlist::{block::WaveformCache, toolbar::PlaylistTool},
    },
    primitives::{NO_RADIUS, PAD_8, PAD_16, PAD_32, PAD_64},
    regions::safe_scissor,
};
use crate::project::{AudioBlock, AudioBlockID, PatternData, Track};

pub mod block;
pub mod grid;
pub mod playhead;
pub mod ruler;
pub mod toolbar;

/// Draws the playlist directly into its four clipped frame regions.
pub fn draw(
    window: &MiniWindow,
    audio_blocks: &[AudioBlock],
    patterns: &[PatternData],
    tracks: &[Track],
    mouse_state: &MouseState,
    playlist_draw: PlaylistAction,
    scroll_offset: &ScrollOffset,
    playhead_beat: f32,
    resizing_audio_block: Option<AudioBlockID>,
    dragging_file: Option<&PathBuf>,
    playlist_tool: &PlaylistTool,
    waveform_cache: &mut WaveformCache,
    stretch_mode: bool,
    frame: &mut FrameDrawContext<'_>,
) -> InteractionResult {
    let screen_config = *frame.screen_config();
    let sw = screen_config.width;
    let sh = screen_config.height;
    let wx = (window.x.max(0.0) as u32).min(sw);
    let wy = ((window.y - TITLEBAR_HEIGHT).max(0.0) as u32).min(sh);
    let win_right = ((window.x + window.width) as u32).min(sw);
    let win_bottom = ((window.y + window.height) as u32).min(sh);
    let content_y = (window.y as u32 + PAD_64 as u32).min(sh);
    let content_h = win_bottom.saturating_sub(content_y);
    let header_x = ((window.x + PAD_16 + grid::GRID_X_ORIGIN).max(0.0) as u32).min(sw);
    let header_w = header_x.saturating_sub(wx);
    let timeline_w = win_right.saturating_sub(header_x);
    let ruler_y = content_y.saturating_sub(24);
    let ruler_h = content_y.saturating_sub(ruler_y);
    let ruler_padding = 16.0;
    let ruler_x = (header_x as f32 - ruler_padding).max(0.0) as u32;
    let ruler_w = timeline_w + ruler_padding as u32;

    let step_count = 64;
    let track_count = 32;
    let mut titlebar_interaction = None;
    let mut toolbar_interaction = None;
    let mut timeline_interaction = None;
    let mut checkbox_clicked = false;

    frame.with_region(
        Some(safe_scissor(
            wx,
            wy,
            win_right.saturating_sub(wx),
            win_bottom.saturating_sub(wy),
            sw,
            sh,
        )),
        |frame| {
            window.background().draw(
                &screen_config,
                screen_config.palette.neutral.shade(3),
                [0.0, 16.0, 0.0, 16.0],
                frame,
            );
            let (title_text, title_interaction) =
                window.title_bar("Playlist", &screen_config, mouse_state, frame);
            frame.draw_texts(&[title_text]);
            titlebar_interaction = Some(title_interaction);

            let tool_interaction =
                toolbar::draw(window, &screen_config, mouse_state, playlist_tool, frame);
            toolbar_interaction = Some(tool_interaction);

            let mut checkbox_text = Vec::new();
            let stretch_checkbox = checkbox(
                window.x + PAD_64 + PAD_32,
                window.y + PAD_32 + PAD_8,
                stretch_mode,
                mouse_state,
                &screen_config,
                frame,
                &mut checkbox_text,
            );
            frame.draw_texts(&checkbox_text);
            if stretch_checkbox.hovered && mouse_state.left_clicked {
                checkbox_clicked = true;
            }
        },
    );

    frame.with_region(
        Some(safe_scissor(ruler_x, ruler_y, ruler_w, ruler_h, sw, sh)),
        |frame| ruler::draw(window, &screen_config, step_count, scroll_offset, frame),
    );

    frame.with_region(
        Some(safe_scissor(wx, content_y, header_w, content_h, sw, sh)),
        |frame| {
            grid::draw_track_headers(window, track_count, &screen_config, scroll_offset, frame);
        },
    );

    frame.with_region(
        Some(safe_scissor(
            header_x, content_y, timeline_w, content_h, sw, sh,
        )),
        |frame| {
            let grid_interaction = grid::draw_grid(
                window,
                track_count,
                step_count,
                &screen_config,
                mouse_state,
                scroll_offset,
                dragging_file,
                playlist_draw,
                patterns,
                frame,
            );

            let mut block_interaction = InteractionResult::default();
            for audio_block in audio_blocks {
                block_interaction = block_interaction.or(block::draw_audio_block(
                    tracks,
                    scroll_offset,
                    audio_block,
                    window,
                    mouse_state,
                    &screen_config,
                    patterns,
                    resizing_audio_block,
                    waveform_cache,
                    frame,
                ));
            }
            // Blocks retain priority over clicking an empty grid cell.
            timeline_interaction = Some((block_interaction, grid_interaction));

            playhead::draw(playhead_beat, window, scroll_offset).draw(
                &screen_config,
                screen_config.palette.secondary.shade(5),
                NO_RADIUS,
                frame,
            );
        },
    );

    let (block_interaction, grid_interaction) = timeline_interaction.unwrap_or_default();
    let mut interaction = titlebar_interaction
        .unwrap_or_default()
        .or(block_interaction)
        .or(grid_interaction.or(toolbar_interaction.unwrap_or_default()));
    if checkbox_clicked {
        interaction.click = ClickResult::ToggleStretchMode;
    }
    interaction
}
