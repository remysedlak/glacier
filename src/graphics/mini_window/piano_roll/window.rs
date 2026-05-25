use crate::app::{MouseState, PianoRollState};
use crate::graphics::{
    color::*,
    font::{TextItem, MONO_FONT},
    mini_window::{piano_roll::*, MiniWindow},
    primitives::{ScreenConfig, Vertex, BOTTOM_RADIUS_16, NO_RADIUS, PAD_16, PAD_2, PAD_32, PAD_4, PAD_8},
    widgets::{window_background, window_title_bar, Rectangle, ICON_SIZE, TITLEBAR_HEIGHT},
    ClickResult,
};
use crate::project::{Instrument, PatternData, Sequence};
use winit::window::CursorIcon;

pub fn draw(
    window: &MiniWindow,
    mouse_state: &MouseState,
    scroll_x: f32,
    scroll_y: f32,
    screen_config: &ScreenConfig,
    patterns: &[PatternData],
    instruments: &[Instrument],
    active_step: usize,
    piano_roll_state: Option<&PianoRollState>,
) -> (
    Vec<Vertex>, // static vertices (background, titlebar, toolbar)
    Vec<TextItem>,
    Vec<Vertex>, // piano key vertices
    Vec<TextItem>,
    Vec<Vertex>, // grid vertices
    Vec<TextItem>,
    ClickResult,
    CursorIcon,
) {
    let mut static_vertices: Vec<Vertex> = Vec::new();
    let mut static_text_items: Vec<TextItem> = Vec::new();
    //
    let mut piano_key_vertices: Vec<Vertex> = Vec::new();
    let mut piano_key_text_items: Vec<TextItem> = Vec::new();
    //
    let mut grid_vertices: Vec<Vertex> = Vec::new();
    let mut grid_text_items: Vec<TextItem> = Vec::new();
    //
    let mut cursor_icon = CursorIcon::Default;
    let mut click_result = ClickResult::None;

    let playlist_background = window_background(&window);
    static_vertices.extend(playlist_background.draw(&screen_config, MINI_WINDOW_BACKGROUND, BOTTOM_RADIUS_16));

    // titlebar
    let title = if let Some(state) = piano_roll_state {
        if let Some(instrument) = instruments.iter().find(|i| i.data.id == state.instrument_id) {
            format!("Piano Roll - {}", instrument.data.name)
        } else {
            "Piano Roll".to_string()
        }
    } else {
        "Piano Roll".to_string()
    };
    let (titlebar_verts, titlebar_texts, result, cursor) = window_title_bar(&window, &title, screen_config, mouse_state);
    if !matches!(cursor, CursorIcon::Default) {
        cursor_icon = cursor;
    }
    click_result = click_result.or(result);
    static_vertices.extend(titlebar_verts);
    static_text_items.push(titlebar_texts);

    // toolbar
    let bottom_toolbar_background = Rectangle {
        x: window.x + SEMITONE_OFFSET_X + PIANO_ROLL_WIDTH,
        y: window.y + TITLEBAR_HEIGHT + PAD_8,
        width: window.width - PAD_16 - PAD_8 - PIANO_ROLL_WIDTH,
        height: 24.0,
    };
    static_vertices.extend(bottom_toolbar_background.draw(&screen_config, LL_GRAY, NO_RADIUS));
    for icon in 0..8 {
        let icon_rect = Rectangle {
            x: window.x + SEMITONE_OFFSET_X + PIANO_ROLL_WIDTH + (icon as f32 * 36.0) + PAD_4,
            y: window.y + PAD_4,
            width: ICON_SIZE,
            height: ICON_SIZE,
        };
        let _hovered = icon_rect.is_hovered(mouse_state.x, mouse_state.y);
        static_vertices.extend(icon_rect.draw(screen_config, DARK_GRAY, NO_RADIUS));
    }

    // find the sequence for the current pattern and instrument, if it exists
    let sequence: Option<&Sequence> = piano_roll_state.and_then(|state| {
        patterns
            .iter()
            .find(|p| p.id == state.pattern_id)
            .and_then(|p| p.sequences.iter().find(|s| s.instrument_id == state.instrument_id))
    });

    // piano roll keys and grid
    for octave in 0..9 {
        for semitone in 0..12 {
            if BLACK_SEMITONE_INDEXES.contains(&semitone) {
                let white_key_width = 24.0;
                let black_key_width = PIANO_ROLL_WIDTH - white_key_width;
                let black_piano_key = Rectangle {
                    x: window.x + SEMITONE_OFFSET_X,
                    y: window.y + (semitone as f32 * SEMITONE_GAP) + (octave as f32 * OCTAVE_GAP) + PIANO_ROLL_MARGIN + PAD_8 - scroll_y,
                    height: SEMITONE_HEIGHT,
                    width: black_key_width,
                };
                let white_piano_key = Rectangle {
                    x: window.x + black_key_width + SEMITONE_OFFSET_X,
                    y: window.y + (semitone as f32 * SEMITONE_GAP) + (octave as f32 * OCTAVE_GAP) + PIANO_ROLL_MARGIN + PAD_8 - scroll_y,
                    height: SEMITONE_HEIGHT,
                    width: white_key_width,
                };

                // is either or both of the black or white piano key being hovered
                let piano_hover =
                    black_piano_key.is_hovered(mouse_state.x, mouse_state.y) || white_piano_key.is_hovered(mouse_state.x, mouse_state.y);
                let white_hover_color = if piano_hover { ORANGE } else { WHITE };
                let black_hover_color = if piano_hover { DARK_GRAY } else { BLACK };

                // add both parts of the key
                piano_key_vertices.extend(black_piano_key.draw(screen_config, black_hover_color, [2.0, 2.0, 2.0, 2.0]));
                piano_key_vertices.extend(white_piano_key.draw(screen_config, white_hover_color, [2.0, 2.0, 2.0, 2.0]));
            } else {
                let piano_key = Rectangle {
                    x: window.x + SEMITONE_OFFSET_X,
                    y: window.y + (semitone as f32 * SEMITONE_GAP) + (octave as f32 * OCTAVE_GAP) + PIANO_ROLL_MARGIN + PAD_8 - scroll_y,
                    height: SEMITONE_HEIGHT,
                    width: PIANO_ROLL_WIDTH,
                };
                piano_key_vertices.extend(piano_key.draw(
                    screen_config,
                    white_piano_step_hover_color(&piano_key, mouse_state.x, mouse_state.y, semitone),
                    [2.0, 2.0, 2.0, 2.0],
                ));
            }

            // draw steps
            for step_index in 0..127 {
                let piano_roll_step = Rectangle {
                    x: window.x + (step_index as f32 * PAD_32) + PIANO_ROLL_MARGIN + SEMITONE_OFFSET_X - scroll_x,
                    y: window.y + (semitone as f32 * SEMITONE_GAP) + (octave as f32 * OCTAVE_GAP) + PIANO_ROLL_MARGIN + PAD_8 - scroll_y,
                    height: SEMITONE_HEIGHT,
                    width: 30.0,
                };

                if piano_roll_step.y + piano_roll_step.height < window.y || piano_roll_step.y > window.y + window.height {
                    break;
                }
                if piano_roll_step.x + piano_roll_step.width < window.x {
                    continue;
                }
                if piano_roll_step.x > window.x + window.width {
                    break;
                }

                let midi_note = ((8 - octave) * 12 + (11 - semitone)) as u8;
                let is_active = sequence
                    .and_then(|s| s.steps.get(step_index))
                    .map(|n| n.velocity > 0.0 && n.pitch == midi_note)
                    .unwrap_or(false);

                let hovered = piano_roll_step.is_hovered(mouse_state.x, mouse_state.y) && !mouse_state.left_click_held;

                let color = if is_active {
                    ORANGE
                } else if hovered {
                    if (step_index / 4) % 2 == 0 {
                        BLUE_HOVER
                    } else {
                        DARK_BLUE_HOVER
                    }
                } else {
                    if (step_index / 4) % 2 == 0 {
                        BLUE
                    } else {
                        DARK_BLUE
                    }
                };

                if piano_roll_step.is_hovered(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
                    if let Some(state) = piano_roll_state {
                        click_result = ClickResult::ToggleNote(state.pattern_id, state.instrument_id, step_index, midi_note);
                    }
                }

                grid_vertices.extend(piano_roll_step.draw(screen_config, color, NO_RADIUS));
            }
        } // end semitone loop

        // push label for C note
        let c_label = format!("C{}", 8 - octave);
        piano_key_text_items.push(TextItem {
            text: c_label.to_string(),
            x: window.x + PAD_32 + PAD_16 + PAD_2 + PAD_8,
            y: window.y + (11.0 * SEMITONE_GAP) + (octave as f32 * OCTAVE_GAP) + PIANO_ROLL_MARGIN + PAD_8 - scroll_y,
            size: 10.0,
            font: MONO_FONT,
            color: BLACK,
        });
    } // end octave loop

    // active step line
    let active_step_line = Rectangle {
        x: window.x + (active_step as f32 * PAD_32) + PIANO_ROLL_MARGIN + SEMITONE_OFFSET_X - scroll_x,
        y: window.y + PIANO_ROLL_MARGIN + PAD_8 - scroll_y,
        width: 4.0,
        height: 9.0 * OCTAVE_GAP,
    };
    grid_vertices.extend(active_step_line.draw(screen_config, GREEN, NO_RADIUS));
    (
        static_vertices,
        static_text_items,
        piano_key_vertices,
        piano_key_text_items,
        grid_vertices,
        grid_text_items,
        click_result,
        cursor_icon,
    )
}
