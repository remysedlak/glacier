use crate::app::MouseState;
use crate::graphics::{
    color::*,
    font::{truncate_text, ROBOTO},
    icons::IconDraw,
    mini_window::MiniWindow,
    primitives::*,
    widgets::window_title_bar,
    {ClickResult, Rectangle, ScrollOffset, TextItem},
};
use crate::project::{Note, PatternData, Track};
use winit::window::CursorIcon;

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

pub fn draw(
    window: &MiniWindow,
    patterns: &[PatternData],
    scroll_offset: &ScrollOffset,
    tracks: &mut [Track],
    active_pattern_id: usize,
    active_step: usize,
    mouse_state: &MouseState,
    screen_config: &ScreenConfig,
    out: &mut Vec<Vertex>,
) -> (Vec<TextItem>, Vec<IconDraw>, ClickResult, CursorIcon) {
    // setup
    let mut text_items: Vec<TextItem> = Vec::new();
    let mut click_result = ClickResult::None;
    let mut cursor_icon = CursorIcon::Default;
    let icons: Vec<IconDraw> = Vec::new();

    // window background
    let window_background = Rectangle {
        x: window.x,
        y: window.y,
        width: window.width,
        height: window.height,
    };
    window_background.draw(screen_config, MINI_WINDOW_BACKGROUND, BOTTOM_RADIUS_16, out);

    // titlebar
    let (titlebar_texts, result, cursor) =
        window_title_bar(window, &window.title, screen_config, mouse_state, out);
    if !matches!(cursor, CursorIcon::Default) {
        cursor_icon = cursor;
    }
    click_result = click_result.or(result);

    text_items.push(titlebar_texts);

    // collect steps values for each row
    let steps_data: Vec<(u32, &[Note])> = patterns
        .get(active_pattern_id)
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
                background.draw(screen_config, DARK_GRAY, NO_RADIUS, out);
                let bar = Rectangle {
                    x: step_x,
                    y: y + SEQUENCER_STEP_HEIGHT - filled_height,
                    width: SEQUENCER_STEP_WIDTH,
                    height: filled_height,
                };
                bar.draw(screen_config, BLUE, NO_RADIUS, out);
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
                let is_ghost = j as usize >= steps_slice.len();
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
                let mut step_color = if j == active_step as u32 && hovered {
                    BLUE_HOVER
                } else if j == active_step as u32 {
                    BLUE
                } else if hovered && is_active {
                    DARK_GRAY
                } else if hovered {
                    LL_GRAY
                } else if is_active {
                    BLACK
                } else {
                    WHITE
                };
                if is_ghost {
                    step_color = if hovered { LL_GRAY } else { GHOST };
                }
                step_button.draw(screen_config, step_color, RADIUS_4, out);

                // check if the step was clicked
                if hovered && mouse_state.left_clicked {
                    click_result = ClickResult::ToggleStep(
                        active_pattern_id,
                        track.data.id as usize,
                        j as usize,
                    );
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
        let track_button_color = if track_button.is_hovered(mouse_state.x, mouse_state.y)
            && !mouse_state.left_click_held
        {
            DARK_GRAY_HOVER
        } else {
            DARK_GRAY
        };
        track_button.draw(screen_config, track_button_color, RADIUS_4, out);
        if track_button.is_hovered(mouse_state.x, mouse_state.y) {
            cursor_icon = CursorIcon::Pointer;
            if mouse_state.left_clicked {
                click_result = ClickResult::ToggleTrackWindow(i);
            }
            if mouse_state.right_clicked {
                click_result = ClickResult::OpenTrackMenu(
                    track_button_x,
                    track_button_y,
                    active_pattern_id,
                    i,
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
        let mute_button_color = if hovered && track.data.is_muted {
            ORANGE_HOVER
        } else if hovered {
            LL_GRAY
        } else if track.data.is_muted {
            ORANGE
        } else {
            LIGHT_GRAY
        };
        mute_button.draw(screen_config, mute_button_color, RADIUS_4, out);

        text_items.push(TextItem {
            text: "mut".to_string(),
            x: window.x + PAD_8 + PAD_4,
            y: window.y + i as f32 * TRACK_GAP + ACTIONS_Y_OFFSET + PAD_2,
            size: 14.0,
            color: BLACK,
            font: ROBOTO,
        });

        if mute_button.is_hovered(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
            track.data.is_muted = !track.data.is_muted;
            click_result = ClickResult::ToggleTrackMute(i);
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
        let velocity_button_color = if hovered && track.show_velocity {
            ORANGE_HOVER
        } else if hovered {
            LL_GRAY
        } else if track.show_velocity {
            ORANGE
        } else {
            LIGHT_GRAY
        };
        velocity_button.draw(screen_config, velocity_button_color, RADIUS_4, out);

        text_items.push(TextItem {
            text: "vel".to_string(),
            x: PAD_16 + window.x + 32.0 + PAD_4,
            y: window.y + i as f32 * TRACK_GAP + ACTIONS_Y_OFFSET + PAD_2,
            size: 14.0,
            color: BLACK,
            font: ROBOTO,
        });
        if velocity_button.is_hovered(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
            track.show_velocity = !track.show_velocity;
        };

        // track volume knob
        draw_knob(
            window.x + KNOB_OFFSET,
            window.y + (i as f32 * TRACK_GAP) + ACTIONS_Y_OFFSET + PAD_8,
            track.data.track_volume,
            screen_config,
            out,
        );

        // track name

        text_items.push(TextItem {
            text: truncate_text(&track.data.name, 23),
            x: window.x + PAD_16,
            y: window.y + i as f32 * TRACK_GAP + PAD_16 + PAD_4,
            size: 12.0,
            color: WHITE,
            font: ROBOTO,
        });
    }

    (text_items, icons, click_result, cursor_icon)
}
