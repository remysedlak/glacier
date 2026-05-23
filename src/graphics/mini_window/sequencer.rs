use crate::app::MouseState;

use crate::graphics::color::{DARK_GRAY_HOVER, LIGHT_GRAY, LL_GRAY, ORANGE, ORANGE_HOVER};
use crate::graphics::font::ROBOTO_FONT;
use crate::graphics::primitives::{BOTTOM_RADIUS_16, NO_RADIUS};
use crate::graphics::{
    color::{BLACK, BLUE, DARK_GRAY, WHITE},
    icons::IconDraw,
    mini_window::{MiniWindow, MINI_WINDOW_BACKGROUND},
    primitives::{draw_knob, ScreenConfig, Vertex, BUTTON_GAP, PAD_16, PAD_2, PAD_4, PAD_8},
    widgets::window_title_bar,
    {ClickResult, Rectangle, TextItem},
};
use crate::project::{Instrument, Note, PatternData, Sequence};
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

pub fn draw(
    window: &MiniWindow,
    patterns: &mut [PatternData],

    instruments: &mut [Instrument],
    active_pattern_id: usize,
    active_step: usize,
    mouse_state: &MouseState,
    screen_config: &ScreenConfig,
) -> (Vec<Vertex>, Vec<TextItem>, Vec<IconDraw>, ClickResult, CursorIcon) {
    // buckets
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut text_items: Vec<TextItem> = Vec::new();
    let mut click_result = ClickResult::None;
    let mut cursor_icon = CursorIcon::Default;
    let icons: Vec<IconDraw> = Vec::new();

    //  size: 18.0, window background
    let window_background = Rectangle {
        x: window.x,
        y: window.y,
        width: window.width,
        height: window.height + 52.0 * instruments.len() as f32,
    };
    vertices.extend(window_background.draw(&screen_config, MINI_WINDOW_BACKGROUND, BOTTOM_RADIUS_16));

    // titlebar
    let (titlebar_verts, titlebar_texts, result, cursor) = window_title_bar(&window, &screen_config, &mouse_state);
    if !matches!(cursor, CursorIcon::Default) {
        cursor_icon = cursor;
    }
    click_result = click_result.or(result);
    vertices.extend(titlebar_verts);
    text_items.push(titlebar_texts);

    // collect steps values for each row
    let steps_data: Vec<(u32, Vec<Note>)> = patterns
        .get(active_pattern_id)
        .map(|p| p.sequences.iter().map(|s| (s.instrument_id, s.steps.clone())).collect())
        .unwrap_or_default();

    // render ever instrument for every pattern
    for (i, instrument) in instruments.iter_mut().enumerate() {
        let y = PAD_16 + window.y + (i as f32 * TRACK_GAP);
        let empty = vec![Note::default(); 32];
        let steps_slice: &[Note] = steps_data
            .iter()
            .find(|(id, _)| *id == instrument.data.id)
            .map(|(_, s)| s.as_slice())
            .unwrap_or(&empty);

        // velocity view
        if instrument.show_velocity {
            // for each step
            for (j, step) in steps_slice.iter().enumerate() {
                // velocity bar
                let step_x = SEQUENCER_X_ORIGIN + window.x + (j as f32 * BUTTON_GAP) + ((j / 4) as f32 * BAR_GAP) + PAD_16;
                let filled_height = SEQUENCER_STEP_HEIGHT * (step.velocity / 128.0);

                // background
                let background = Rectangle {
                    x: step_x,
                    y,
                    width: SEQUENCER_STEP_WIDTH,
                    height: SEQUENCER_STEP_HEIGHT,
                };
                vertices.extend(background.draw(&screen_config, DARK_GRAY, NO_RADIUS));
                let bar = Rectangle {
                    x: step_x,
                    y: y + SEQUENCER_STEP_HEIGHT - filled_height,
                    width: SEQUENCER_STEP_WIDTH,
                    height: filled_height,
                };
                vertices.extend(bar.draw(&screen_config, BLUE, NO_RADIUS));
            }
        }
        // steps view
        else {
            for (j, step) in steps_slice.iter().enumerate() {
                // add the button for a step
                let step_x = SEQUENCER_X_ORIGIN + window.x + (j as f32 * BUTTON_GAP) + ((j / 4) as f32 * BAR_GAP) + PAD_16;
                let step_button = Rectangle {
                    x: step_x,
                    y,
                    width: SEQUENCER_STEP_WIDTH,
                    height: SEQUENCER_STEP_HEIGHT,
                };
                let is_active = step.velocity > 0.0;
                let step_color = if step_button.is_hovered(mouse_state.x, mouse_state.y) && is_active {
                    DARK_GRAY
                } else if step_button.is_hovered(mouse_state.x, mouse_state.y) {
                    LL_GRAY
                } else if is_active {
                    BLACK
                } else {
                    WHITE
                };
                vertices.extend(step_button.draw(screen_config, step_color, NO_RADIUS));

                // check if the step was clicked
                if step_button.is_hovered(mouse_state.x, mouse_state.y) {
                    if mouse_state.left_clicked {
                        // if the click is on an existing sequence
                        if let Some(seq) = patterns[active_pattern_id]
                            .sequences
                            .iter_mut()
                            .find(|s| s.instrument_id == instrument.data.id)
                        {
                            seq.steps[j] = if seq.steps[j].velocity > 0.0 {
                                Note::default()
                            } else {
                                Note {
                                    velocity: 95.0,
                                    ..Default::default()
                                }
                            };
                        }
                        // if the click is on a nonexisting sequence
                        else {
                            // add a new sequence to the active pattern with the instrument used
                            let mut steps = vec![Note::default(); 32];
                            steps[j] = Note {
                                velocity: 95.0,
                                ..Default::default()
                            };
                            patterns[active_pattern_id].sequences.push(Sequence {
                                instrument_id: instrument.data.id,
                                steps,
                            });
                        }
                        click_result = ClickResult::Step(active_pattern_id, instrument.data.id as usize, j);
                    }
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
        let track_button_color = if track_button.is_hovered(mouse_state.x, mouse_state.y) && !mouse_state.left_click_held {
            DARK_GRAY_HOVER
        } else {
            DARK_GRAY
        };
        vertices.extend(track_button.draw(&screen_config, track_button_color, NO_RADIUS));
        if track_button.is_hovered(mouse_state.x, mouse_state.y) {
            cursor_icon = CursorIcon::Pointer;
            if mouse_state.left_clicked {
                click_result = ClickResult::ToggleInstrumentWindow(i);
            }
            if mouse_state.right_clicked {
                click_result = ClickResult::OpenTrackMenu(track_button_x, track_button_y, i);
            }
        }

        // mute button
        let mute_button = Rectangle {
            x: PAD_8 + window.x,
            y: ACTIONS_Y_OFFSET + PAD_2 + window.y + (i as f32 * TRACK_GAP),
            width: MUTE_SQUARE_LENGTH * 2.0,
            height: MUTE_SQUARE_LENGTH,
        };
        let hovered = mute_button.is_hovered(mouse_state.x, mouse_state.y) && !mouse_state.left_click_held;
        let mute_button_color = if hovered && instrument.data.is_muted {
            ORANGE_HOVER
        } else if hovered {
            LL_GRAY
        } else if instrument.data.is_muted {
            ORANGE
        } else {
            LIGHT_GRAY
        };
        vertices.extend(mute_button.draw(&screen_config, mute_button_color, NO_RADIUS));

        text_items.push(TextItem {
            text: "mut".to_string(),
            x: window.x + PAD_8 + PAD_4,
            y: window.y + i as f32 * TRACK_GAP + ACTIONS_Y_OFFSET + PAD_2,
            size: 14.0,
            color: BLACK,
            font: ROBOTO_FONT,
        });

        if mute_button.is_hovered(mouse_state.x, mouse_state.y) {
            cursor_icon = CursorIcon::Pointer;
            if mouse_state.left_clicked {
                instrument.data.is_muted = !instrument.data.is_muted;
                click_result = ClickResult::Mute(i);
            }
        }

        // velocity button
        let velocity_button = Rectangle {
            x: PAD_16 + window.x + 32.0,
            y: window.y + i as f32 * TRACK_GAP + ACTIONS_Y_OFFSET + PAD_2,
            width: MUTE_SQUARE_LENGTH * 2.0,
            height: MUTE_SQUARE_LENGTH,
        };

        let hovered = velocity_button.is_hovered(mouse_state.x, mouse_state.y) && !mouse_state.left_click_held;
        let velocity_button_color = if hovered && instrument.show_velocity {
            ORANGE_HOVER
        } else if hovered {
            LL_GRAY
        } else if instrument.show_velocity {
            ORANGE
        } else {
            LIGHT_GRAY
        };
        vertices.extend(velocity_button.draw(&screen_config, velocity_button_color, NO_RADIUS));

        text_items.push(TextItem {
            text: "vel".to_string(),
            x: PAD_16 + window.x + 32.0 + PAD_4,
            y: window.y + i as f32 * TRACK_GAP + ACTIONS_Y_OFFSET + PAD_2,
            size: 14.0,
            color: BLACK,
            font: ROBOTO_FONT,
        });
        if velocity_button.is_hovered(mouse_state.x, mouse_state.y) {
            cursor_icon = CursorIcon::Pointer;
            if mouse_state.left_clicked {
                instrument.show_velocity = !instrument.show_velocity;
            }
        }

        // track volume knob
        for vert in draw_knob(
            window.x + KNOB_OFFSET,
            window.y + (i as f32 * TRACK_GAP) + ACTIONS_Y_OFFSET + PAD_8,
            instrument.data.track_volume,
            screen_config,
        ) {
            vertices.push(vert);
        }

        // instrument name
        let instrument_button_text: String = if instrument.data.name.len() > 23 {
            let end = instrument.data.name.floor_char_boundary(23);
            let truncated_name = &instrument.data.name[..end].to_string(); // Safely gets "こん" (6 bytes)
            format!("{}{}", truncated_name, "...",)
        } else {
            instrument.data.name.to_string()
        };

        text_items.push(TextItem {
            text: instrument_button_text,
            x: window.x + PAD_16,
            y: window.y + i as f32 * TRACK_GAP + PAD_16 + PAD_4,
            size: 16.0,
            color: WHITE,
            font: ROBOTO_FONT,
        });
    }

    (vertices, text_items, icons, click_result, cursor_icon)
}
