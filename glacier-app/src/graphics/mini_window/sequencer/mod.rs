use crate::app::{MouseState, click::ClickResult};
use crate::graphics::mini_window::InteractionResult;
use crate::graphics::{
    font::{Font::Roboto, truncate_text},
    mini_window::MiniWindow,
    primitives::*,
    {FrameDrawContext, Rectangle, ScrollOffset, TextItem},
};
use crate::project::{Note, PatternData, PatternID, Track, TrackID};
use winit::window::CursorIcon;

pub mod utils;

pub const BAR_GAP: f32 = 12.0;
pub const KNOB_OFFSET: f32 = 110.0;
pub const KNOB_RADIUS: f32 = 8.0;
pub const ACTIONS_Y_OFFSET: f32 = 44.0;
pub const TRACK_GAP: f32 = 72.0;
pub const SEQUENCER_X_ORIGIN: f32 = 200.0;
pub const SEQUENCER_STEP_WIDTH: f32 = 18.0;
pub const SEQUENCER_STEP_HEIGHT: f32 = 48.0;
pub const MUTE_SQUARE_LENGTH: f32 = 16.0;
pub const MAX_STEPS: u32 = 256;
const STEPS_PER_BAR: usize = 16;

/// Draw the sequencer Mini Window. The user can compose a pattern here or access track context menus
pub fn draw(
    window: &MiniWindow,
    patterns: &[PatternData],
    scroll_offset: &ScrollOffset,
    tracks: &mut [Track],
    active_pattern_id: PatternID,
    active_step: usize,
    mouse_state: &MouseState,
    frame: &mut FrameDrawContext<'_>,
) -> InteractionResult {
    let screen_config = *frame.screen_config();
    let mut interaction = InteractionResult::default();

    // window background
    let window_background = Rectangle {
        x: window.x,
        y: window.y,
        width: window.width,
        height: window.height,
    };
    window_background.draw(
        &screen_config,
        screen_config.palette.neutral.shade(4),
        BOTTOM_RADIUS_16,
        frame,
    );

    // titlebar
    let (titlebar_texts, titlebar_interaction) =
        window.title_bar(&window.title, &screen_config, mouse_state, frame);
    frame.add_interaction(titlebar_interaction);
    frame.draw_texts(&[titlebar_texts]);

    // collect steps values for each row
    let steps_data: Vec<(TrackID, &[Note])> = patterns
        .get(active_pattern_id.0 as usize)
        .map(|p| {
            p.sequences
                .iter()
                .map(|s| (s.track_id, s.steps.as_slice()))
                .collect()
        })
        .unwrap_or_default();

    static EMPTY_STEPS: [Note; 32] = [Note::DEFAULT; 32];
    // render ever track for every pattern
    for (i, track) in tracks.iter_mut().enumerate() {
        let y = PAD_16 + window.y + (i as f32 * TRACK_GAP);

        let steps_slice: &[Note] = steps_data
            .iter()
            .find(|(id, _)| *id == track.data.id)
            .map(|(_, s)| *s)
            .unwrap_or(&EMPTY_STEPS);
        // Keep the final partially filled bar solid; ghosting starts at the next bar.
        let ghost_start =
            steps_slice.len().max(STEPS_PER_BAR).div_ceil(STEPS_PER_BAR) * STEPS_PER_BAR;

        // velocity view
        if track.show_velocity {
            // for each step
            for (j, step) in steps_slice.iter().enumerate() {
                // velocity bar
                let step_x = SEQUENCER_X_ORIGIN
                    + window.x
                    + (j as f32 * BUTTON_GAP)
                    + ((j / 4) as f32 * BAR_GAP)
                    + PAD_16
                    - scroll_offset.x;
                if step_x + SEQUENCER_STEP_WIDTH < window.x + SEQUENCER_X_ORIGIN {
                    continue;
                }
                if step_x > window.x + window.width {
                    break;
                }
                let filled_height = SEQUENCER_STEP_HEIGHT * (step.velocity / 128.0);

                // background
                let background = Rectangle {
                    x: step_x,
                    y,
                    width: SEQUENCER_STEP_WIDTH,
                    height: SEQUENCER_STEP_HEIGHT,
                };
                background.draw(
                    &screen_config,
                    screen_config.palette.neutral.shade(3),
                    NO_RADIUS,
                    frame,
                );
                let bar = Rectangle {
                    x: step_x,
                    y: y + SEQUENCER_STEP_HEIGHT - filled_height,
                    width: SEQUENCER_STEP_WIDTH,
                    height: filled_height,
                };
                bar.draw(
                    &screen_config,
                    screen_config.palette.secondary.shade(4),
                    NO_RADIUS,
                    frame,
                );
            }
        }
        // steps view
        else {
            for j in 0..MAX_STEPS {
                // get the note's song data
                let note = steps_slice
                    .get(j as usize)
                    .copied()
                    .unwrap_or(Note::DEFAULT);
                let is_ghost = j as usize >= ghost_start;
                let is_active = note.velocity > 0.0;

                // add the button for a step
                let step_x = SEQUENCER_X_ORIGIN
                    + window.x
                    + (j as f32 * BUTTON_GAP)
                    + ((j / 4) as f32 * BAR_GAP)
                    + PAD_16
                    - scroll_offset.x;

                // do not paint steps outside of the window boundaries
                if step_x + SEQUENCER_STEP_WIDTH < window.x + SEQUENCER_X_ORIGIN {
                    continue;
                }
                if step_x > window.x + window.width - PAD_16 {
                    break;
                }

                /*
                    draw the note button according to
                    active song step,
                    note velocity,
                    hover state,
                    and recorded track length
                */

                let step_button = Rectangle {
                    x: step_x,
                    y,
                    width: SEQUENCER_STEP_WIDTH,
                    height: SEQUENCER_STEP_HEIGHT,
                };
                let hovered = step_button.is_hovered(mouse_state.x, mouse_state.y); // cache it!
                let active = j == active_step as u32;
                let step_color =
                    utils::step_color(screen_config.palette, hovered, is_active, is_ghost, active);

                step_button.draw(&screen_config, step_color, RADIUS_4, frame);

                // check if the step was clicked
                if hovered && mouse_state.left_clicked {
                    interaction.click =
                        ClickResult::ToggleStep(active_pattern_id, track.data.id, j as usize);
                }
            }
        }

        // ACTIONS FOR EACH TRACK /////////////////

        let track_button_x = PAD_8 + window.x;
        let track_button_y = PAD_16 + window.y + (i as f32 * TRACK_GAP);
        let track_button = Rectangle {
            x: track_button_x,
            y: track_button_y,
            width: 172.0,
            height: 24.0,
        };
        let hovered = track_button.is_hovered(mouse_state.x, mouse_state.y);
        let color_hovered = hovered && !mouse_state.left_click_held;
        let color = if color_hovered {
            screen_config.palette.neutral.shade(3)
        } else {
            screen_config.palette.neutral.shade(2)
        };
        track_button.draw(&screen_config, color, RADIUS_4, frame);
        if hovered {
            interaction.cursor = CursorIcon::Pointer;
            if mouse_state.left_clicked {
                interaction.click = ClickResult::ToggleTrackWindow(track.data.id);
            }
            if mouse_state.right_clicked {
                interaction.click = ClickResult::OpenTrackMenu(
                    track_button_x,
                    track_button_y,
                    active_pattern_id,
                    track.data.id,
                );
            }
        }

        // mute button
        let mute_button = Rectangle {
            x: PAD_8 + window.x,
            y: ACTIONS_Y_OFFSET + PAD_2 + window.y + (i as f32 * TRACK_GAP),
            width: MUTE_SQUARE_LENGTH * 2.0,
            height: MUTE_SQUARE_LENGTH,
        };
        let hovered =
            mute_button.is_hovered(mouse_state.x, mouse_state.y) && !mouse_state.left_click_held;
        let base = if track.data.is_muted {
            screen_config.palette.primary.shade(5)
        } else {
            screen_config.palette.neutral.shade(7)
        };
        let mute_button_color = if hovered {
            screen_config.palette.primary.shade(7)
        } else {
            base
        };
        mute_button.draw(&screen_config, mute_button_color, RADIUS_4, frame);

        frame.draw_texts(&[TextItem {
            text: "mut".to_string(),
            x: window.x + PAD_8 + PAD_4,
            y: window.y + i as f32 * TRACK_GAP + ACTIONS_Y_OFFSET + PAD_2,
            size: 14.0,
            color: screen_config.palette.neutral.shade(0),
            font: Roboto,
        }]);

        if mute_button.is_hovered(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
            track.data.is_muted = !track.data.is_muted;
            interaction.click = ClickResult::ToggleTrackMute(track.data.id);
        };

        // velocity button
        let velocity_button = Rectangle {
            x: PAD_16 + window.x + 32.0,
            y: window.y + i as f32 * TRACK_GAP + ACTIONS_Y_OFFSET + PAD_2,
            width: MUTE_SQUARE_LENGTH * 2.0,
            height: MUTE_SQUARE_LENGTH,
        };

        let hovered = velocity_button.is_hovered(mouse_state.x, mouse_state.y)
            && !mouse_state.left_click_held;
        let base = if track.show_velocity {
            screen_config.palette.primary.shade(5)
        } else {
            screen_config.palette.neutral.shade(7)
        };
        let velocity_button_color = if hovered {
            screen_config.palette.primary.shade(7)
        } else {
            base
        };
        velocity_button.draw(&screen_config, velocity_button_color, RADIUS_4, frame);

        frame.draw_texts(&[TextItem {
            text: "vel".to_string(),
            x: PAD_16 + window.x + 32.0 + PAD_4,
            y: window.y + i as f32 * TRACK_GAP + ACTIONS_Y_OFFSET + PAD_2,
            size: 14.0,
            color: screen_config.palette.neutral.shade(0),
            font: Roboto,
        }]);
        if velocity_button.is_hovered(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
            track.show_velocity = !track.show_velocity;
        };

        // track volume knob
        draw_knob(
            window.x + KNOB_OFFSET,
            window.y + (i as f32 * TRACK_GAP) + ACTIONS_Y_OFFSET + PAD_8,
            track.data.track_volume,
            &screen_config,
            frame,
        );

        // track name

        frame.draw_texts(&[TextItem {
            text: truncate_text(&track.data.name, 23),
            x: window.x + PAD_16,
            y: window.y + i as f32 * TRACK_GAP + PAD_16 + PAD_4,
            size: 12.0,
            color: screen_config.palette.neutral.shade(7),
            font: Roboto,
        }]);
    }

    interaction
}
