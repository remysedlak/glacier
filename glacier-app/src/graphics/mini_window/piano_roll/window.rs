use crate::app::{MouseState, PianoRollState, ScrollOffset, click::ClickResult};
use crate::graphics::components::toolbar::ICON_SIZE;
use crate::graphics::mini_window::InteractionResult;
use crate::graphics::{
    FrameDrawContext,
    font::{Font::Mono, TextItem},
    geometry::Rectangle,
    mini_window::{MiniWindow, TITLEBAR_HEIGHT, piano_roll::*},
    primitives::{BOTTOM_RADIUS_16, NO_RADIUS, PAD_2, PAD_4, PAD_8, PAD_16, PAD_32},
};
use crate::project::{PatternData, Sequence, Track};
use winit::window::CursorIcon;

/// Draws the piano roll directly into its three clipped frame regions.
pub fn draw(
    window: &MiniWindow,
    mouse_state: &MouseState,
    patterns: &[PatternData],
    tracks: &[Track],
    active_step: usize,
    piano_roll_state: Option<&PianoRollState>,
    frame: &mut FrameDrawContext<'_>,
) -> InteractionResult {
    let screen_config = *frame.screen_config();
    let sw = screen_config.width;
    let sh = screen_config.height;
    let wx = (window.x.max(0.0) as u32).min(sw);
    let wy = ((window.y - TITLEBAR_HEIGHT).max(0.0) as u32).min(sh);
    let win_right = ((window.x + window.width) as u32).min(sw);
    let win_bottom = ((window.y + window.height) as u32).min(sh);
    let content_y = (window.y as u32 + 72).min(sh);
    let content_h = win_bottom.saturating_sub(content_y).saturating_sub(32);
    let key_col_right = (window.x + 72.0).max(0.0) as u32;
    let grid_x = key_col_right.min(sw);
    let key_w = grid_x.saturating_sub(wx);
    let grid_w = win_right.saturating_sub(grid_x).saturating_sub(16);
    let mut interaction = InteractionResult::default();

    let title = piano_roll_state
        .and_then(|state| tracks.iter().find(|track| track.data.id == state.track_id))
        .map(|track| format!("Piano Roll - {}", track.data.name))
        .unwrap_or_else(|| "Piano Roll".to_string());

    frame.with_region(
        Some(crate::graphics::regions::safe_scissor(
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
                BOTTOM_RADIUS_16,
                frame,
            );

            let (title_text, title_interaction) =
                window.title_bar(&title, &screen_config, mouse_state, frame);
            frame.add_interaction(title_interaction);
            frame.draw_texts(&[title_text]);

            let toolbar_background = Rectangle {
                x: window.x + SEMITONE_OFFSET_X + PIANO_ROLL_WIDTH,
                y: window.y + TITLEBAR_HEIGHT + PAD_8,
                width: window.width - PAD_16 - PAD_8 - PIANO_ROLL_WIDTH,
                height: 24.0,
            };
            toolbar_background.draw(
                &screen_config,
                screen_config.palette.neutral.shade(6),
                NO_RADIUS,
                frame,
            );

            for icon in 0..8 {
                let icon_rect = Rectangle {
                    x: window.x
                        + SEMITONE_OFFSET_X
                        + PIANO_ROLL_WIDTH
                        + (icon as f32 * 36.0)
                        + PAD_4,
                    y: window.y + PAD_4,
                    width: ICON_SIZE,
                    height: ICON_SIZE,
                };
                let _hovered = icon_rect.is_hovered(mouse_state.x, mouse_state.y);
                icon_rect.draw(
                    &screen_config,
                    screen_config.palette.neutral.shade(3),
                    NO_RADIUS,
                    frame,
                );
            }
        },
    );

    let sequence: Option<&Sequence> = piano_roll_state.and_then(|state| {
        patterns
            .iter()
            .find(|pattern| pattern.id == state.pattern_id)
            .and_then(|pattern| {
                pattern
                    .sequences
                    .iter()
                    .find(|sequence| sequence.track_id == state.track_id)
            })
    });
    let default_scroll = ScrollOffset { x: 0.0, y: 0.0 };
    let scroll = piano_roll_state
        .map(|state| &state.scroll_offset)
        .unwrap_or(&default_scroll);

    frame.with_region(
        Some(crate::graphics::regions::safe_scissor(
            wx, content_y, key_w, content_h, sw, sh,
        )),
        |frame| {
            for octave in 0..9 {
                for semitone in 0..12 {
                    if BLACK_SEMITONE_INDEXES.contains(&semitone) {
                        let white_key_width = 24.0;
                        let black_key_width = PIANO_ROLL_WIDTH - white_key_width;
                        let black_key = Rectangle {
                            x: window.x + SEMITONE_OFFSET_X,
                            y: window.y
                                + (semitone as f32 * SEMITONE_GAP)
                                + (octave as f32 * OCTAVE_GAP)
                                + PIANO_ROLL_MARGIN
                                + PAD_8
                                - scroll.y,
                            height: SEMITONE_HEIGHT,
                            width: black_key_width,
                        };
                        let white_key = Rectangle {
                            x: window.x + black_key_width + SEMITONE_OFFSET_X,
                            y: black_key.y,
                            height: SEMITONE_HEIGHT,
                            width: white_key_width,
                        };
                        let hovered = black_key.is_hovered(mouse_state.x, mouse_state.y)
                            || white_key.is_hovered(mouse_state.x, mouse_state.y);
                        let white_color = if hovered {
                            screen_config.palette.primary.shade(5)
                        } else {
                            screen_config.palette.neutral.shade(7)
                        };
                        let black_color = if hovered {
                            screen_config.palette.neutral.shade(3)
                        } else {
                            screen_config.palette.neutral.shade(0)
                        };
                        black_key.draw(&screen_config, black_color, [2.0; 4], frame);
                        white_key.draw(&screen_config, white_color, [2.0; 4], frame);
                    } else {
                        let key = Rectangle {
                            x: window.x + SEMITONE_OFFSET_X,
                            y: window.y
                                + (semitone as f32 * SEMITONE_GAP)
                                + (octave as f32 * OCTAVE_GAP)
                                + PIANO_ROLL_MARGIN
                                + PAD_8
                                - scroll.y,
                            height: SEMITONE_HEIGHT,
                            width: PIANO_ROLL_WIDTH,
                        };
                        let color = white_piano_step_hover_color(
                            &key,
                            mouse_state.x,
                            mouse_state.y,
                            semitone,
                            &screen_config,
                        );
                        key.draw(&screen_config, color, [2.0; 4], frame);
                    }
                }

                frame.draw_texts(&[TextItem {
                    text: format!("C{}", 8 - octave),
                    x: window.x + PAD_32 + PAD_16 + PAD_2 + PAD_8,
                    y: window.y
                        + (11.0 * SEMITONE_GAP)
                        + (octave as f32 * OCTAVE_GAP)
                        + PIANO_ROLL_MARGIN
                        + PAD_8
                        - scroll.y,
                    size: 10.0,
                    font: Mono,
                    color: screen_config.palette.neutral.shade(0),
                }]);
            }
        },
    );

    frame.with_region(
        Some(crate::graphics::regions::safe_scissor(
            grid_x, content_y, grid_w, content_h, sw, sh,
        )),
        |frame| {
            for octave in 0..9 {
                for semitone in 0..12 {
                    for step_index in 0..127 {
                        let step = Rectangle {
                            x: window.x
                                + (step_index as f32 * PAD_32)
                                + PIANO_ROLL_MARGIN
                                + SEMITONE_OFFSET_X
                                - scroll.x,
                            y: window.y
                                + (semitone as f32 * SEMITONE_GAP)
                                + (octave as f32 * OCTAVE_GAP)
                                + PIANO_ROLL_MARGIN
                                + PAD_8
                                - scroll.y,
                            height: SEMITONE_HEIGHT,
                            width: 30.0,
                        };

                        if step.y + step.height < window.y || step.y > window.y + window.height {
                            break;
                        }
                        if step.x + step.width < window.x {
                            continue;
                        }
                        if step.x > window.x + window.width {
                            break;
                        }

                        let midi_note = ((8 - octave) * 12 + (11 - semitone)) as u8;
                        let is_active = sequence
                            .and_then(|sequence| sequence.steps.get(step_index))
                            .map(|note| note.velocity > 0.0 && note.pitch == midi_note)
                            .unwrap_or(false);

                        if step.is_hovered_right_edge(mouse_state.x, mouse_state.y) && is_active {
                            interaction.cursor = CursorIcon::ColResize;
                        }

                        let base = if (step_index / 4) % 2 == 0 {
                            screen_config.palette.primary.shade(3)
                        } else {
                            screen_config.palette.primary.shade(4)
                        };
                        let color = if is_active {
                            screen_config.palette.primary.shade(5)
                        } else {
                            base
                        };

                        if step.is_hovered(mouse_state.x, mouse_state.y) && mouse_state.left_clicked
                        {
                            if let Some(state) = piano_roll_state {
                                interaction.click = ClickResult::ToggleNote(
                                    state.pattern_id,
                                    state.track_id,
                                    step_index,
                                    midi_note,
                                );
                            }
                        }

                        step.draw(&screen_config, color, NO_RADIUS, frame);
                    }
                }
            }

            let active_step_line = Rectangle {
                x: window.x + (active_step as f32 * PAD_32) + PIANO_ROLL_MARGIN + SEMITONE_OFFSET_X
                    - scroll.x,
                y: window.y + PIANO_ROLL_MARGIN + PAD_8 - scroll.y,
                width: 4.0,
                height: 9.0 * OCTAVE_GAP,
            };
            active_step_line.draw(
                &screen_config,
                screen_config.palette.primary.shade(4),
                NO_RADIUS,
                frame,
            );
        },
    );

    interaction
}
