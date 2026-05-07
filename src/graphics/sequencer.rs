use crate::color::*;
use crate::graphics::ui::*;
use crate::graphics::{ClickResult, ScreenConfig};
use crate::graphics::{Vertex, BACKGROUND};
use crate::project::*;

pub fn draw(
    window: &MiniWindow,
    patterns: &mut [PatternData],

    instruments: &mut [Instrument],
    active_pattern_id: usize,
    active_step: usize,
    mouse_state: &MouseState,
    screen_config: &ScreenConfig,
) -> (Vec<Vertex>, Vec<(String, f32, f32)>, ClickResult) {
    let padding = 16.0;
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut text_items: Vec<(String, f32, f32)> = Vec::new();
    let mut click_result = ClickResult::None;
    // background of sequencer
    let window_background = Rectangle {
        x: window.x,
        y: window.y,
        width: window.width,
        height: window.height,
    };
    vertices.extend(window_background.draw(&screen_config, BACKGROUND));

    // titlebar rectangle
    let titlebar = Rectangle {
        x: window.x,
        y: window.y - TITLEBAR_HEIGHT,
        width: window.width,
        height: TITLEBAR_HEIGHT,
    };
    vertices.extend(titlebar.draw(&screen_config, DARK_GRAY));

    text_items.push((window.title.to_string(), window.x + window.width / 2.2, window.y - TITLEBAR_HEIGHT + 4.0));

    let steps_data: Vec<(u32, Vec<f32>)> = patterns
        .get(active_pattern_id)
        .map(|p| p.sequences.iter().map(|s| (s.instrument_id, s.steps.clone())).collect())
        .unwrap_or_default();

    // render ever instrument for every pattern
    for (i, instrument) in instruments.iter_mut().enumerate() {
        let y = padding + window.y + (i as f32 * TRACK_GAP);
        let empty = vec![0.0f32; 32];
        let steps_slice: &[f32] = steps_data
            .iter()
            .find(|(id, _)| *id == instrument.data.id)
            .map(|(_, s)| s.as_slice())
            .unwrap_or(&empty);

        // velocity view
        if instrument.show_velocity {
            // for each step
            for (j, &velocity) in steps_slice.iter().enumerate() {
                // velocity bar
                let step_x = 240.0 + window.x + (j as f32 * BUTTON_GAP) + ((j / 4) as f32 * BAR_GAP) + padding;
                let filled_height = BUTTON_HEIGHT * (velocity / 128.0);
                // background
                let background = Rectangle {
                    x: step_x,
                    y,
                    width: BUTTON_WIDTH,
                    height: BUTTON_HEIGHT,
                };
                vertices.extend(background.draw(&screen_config, DARK_GRAY));
                let bar = Rectangle {
                    x: step_x,
                    y: y + BUTTON_HEIGHT - filled_height,
                    width: BUTTON_WIDTH,
                    height: filled_height,
                };
                vertices.extend(bar.draw(&screen_config, BLUE));
            }
        }
        // steps view
        else {
            for (j, &velocity) in steps_slice.iter().enumerate() {
                // add the button for a step
                let step_x = 240.0 + window.x + (j as f32 * BUTTON_GAP) + ((j / 4) as f32 * BAR_GAP) + padding;
                let step = Rectangle {
                    x: step_x,
                    y,
                    width: BUTTON_WIDTH,
                    height: BUTTON_HEIGHT,
                };
                vertices.extend(step.draw(
                    screen_config,
                    step.active_step_color(mouse_state.x, mouse_state.y, j == active_step, velocity > 0.0),
                ));

                // check if the step was clicked
                if mouse_state.left_clicked && step.is_hovered(mouse_state.x, mouse_state.y) {
                    // if the click is on an existing sequence
                    if let Some(seq) = patterns[active_pattern_id]
                        .sequences
                        .iter_mut()
                        .find(|s| s.instrument_id == instrument.data.id)
                    {
                        seq.steps[j] = if seq.steps[j] > 0.0 { 0.0 } else { 95.0 };
                    }
                    // if the click is on a nonexisting sequence
                    else {
                        // add a new sequence to the active pattern with the instrument used
                        let mut steps = vec![0.0f32; 32];
                        steps[j] = 95.0;
                        patterns[active_pattern_id].sequences.push(Sequence {
                            instrument_id: instrument.data.id,
                            steps,
                        });
                    }
                    click_result = ClickResult::Step(active_pattern_id, instrument.data.id as usize, j);
                }
            }
        }

        let button_gap = 40.0;

        // mute button
        let mute_button = Rectangle {
            x: padding + window.x,
            y: 32.0 + window.y + (i as f32 * TRACK_GAP),
            width: MUTE_SQUARE_LENGTH,
            height: MUTE_SQUARE_LENGTH,
        };
        vertices.extend(mute_button.draw(
            &screen_config,
            mute_button.active_color(mouse_state.x, mouse_state.y, instrument.data.is_muted),
        ));

        text_items.push(("mut".to_string(), window.x + 16.0, window.y + i as f32 * TRACK_GAP + 40.0));

        if mouse_state.left_clicked && mute_button.is_hovered(mouse_state.x, mouse_state.y) {
            instrument.data.is_muted = !instrument.data.is_muted;
            click_result = ClickResult::Mute(i);
        }

        // velocity button
        let velocity_button = Rectangle {
            x: padding + window.x + button_gap,
            y: 32.0 + window.y + (i as f32 * TRACK_GAP),
            width: MUTE_SQUARE_LENGTH,
            height: MUTE_SQUARE_LENGTH,
        };
        vertices.extend(velocity_button.draw(
            &screen_config,
            velocity_button.active_color(mouse_state.x, mouse_state.y, instrument.show_velocity),
        ));

        text_items.push(("vel".to_string(), window.x + 50.0, window.y + (i as f32 * TRACK_GAP) + 40.0));
        if mouse_state.left_clicked && velocity_button.is_hovered(mouse_state.x, mouse_state.y) {
            instrument.show_velocity = !instrument.show_velocity;
        }

        // delete button
        let delete_button = Rectangle {
            x: padding + window.x + button_gap + 16.0,
            y: 32.0 + window.y + (i as f32 * TRACK_GAP),
            width: MUTE_SQUARE_LENGTH,
            height: MUTE_SQUARE_LENGTH,
        };
        vertices.extend(delete_button.draw(&screen_config, delete_button.hover_color(mouse_state.x, mouse_state.y)));

        text_items.push((
            "del".to_string(),
            padding + window.x + button_gap + 16.0,
            window.y + (i as f32 * TRACK_GAP) + 40.0,
        ));
        if mouse_state.left_clicked && delete_button.is_hovered(mouse_state.x, mouse_state.y) {
            click_result = ClickResult::DeleteTrack(i);
        }

        // track volume knob
        for vert in draw_knob(
            instrument.data.track_volume,
            window.x + 198.0,
            window.y + (i as f32 * TRACK_GAP) + 24.0,
            KNOB_RADIUS,
            35,
            screen_config,
        ) {
            vertices.push(vert);
        }

        text_items.push((instrument.data.name.to_string(), window.x + 16.0, window.y + i as f32 * TRACK_GAP));
    }
    (vertices, text_items, click_result)
}
