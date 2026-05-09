use crate::graphics::primitives::draw_h_line;
use crate::graphics::primitives::draw_rectangle;
use crate::graphics::widgets::{ADD_INSTRUMENT_ICON_OFFSET, ICON_HEIGHT, ICON_WIDTH, TOOLBAR_MARGIN};
use crate::graphics::Vertex;
use crate::graphics::BUTTON_GAP;
use crate::graphics::LOAD_PROJECT_ICON_OFFSET;
use crate::graphics::TOOLBAR_THICKNESS;
use crate::project::PatternData;
use crate::{
    color::LIGHT_GRAY,
    graphics::{
        ui::{MouseState, PAD_4},
        widgets::{Rectangle, TextItem, PLAY_SQUARE_HEIGHT, PLAY_SQUARE_WIDTH, PLAY_X_ORIGIN, PLAY_Y_ORIGIN, TOOLBAR_Y},
        ClickResult, ScreenConfig,
    },
};
pub fn draw_toolbar(
    mouse_state: &MouseState,
    screen_config: &ScreenConfig,
    mut bpm: f32,
    patterns: &[PatternData],
    mut is_playing: bool,
    active_step: usize,
) -> (Vec<Vertex>, Vec<TextItem>, ClickResult) {
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut click_result = ClickResult::None;
    let bpm_up = Rectangle {
        x: 48.0,
        y: 4.0,
        width: 32.0,
        height: 10.0,
    };
    vertices.extend(bpm_up.draw(&screen_config, LIGHT_GRAY));
    if mouse_state.left_clicked && bpm_up.is_hovered(mouse_state.x, mouse_state.y) {
        bpm += 1.0;
        click_result = ClickResult::ChangeBpm(bpm);
    }

    let bpm_down = Rectangle {
        x: 48.0,
        y: 16.0,
        width: 32.0,
        height: 10.0,
    };
    vertices.extend(bpm_down.draw(&screen_config, LIGHT_GRAY));
    if mouse_state.left_clicked && bpm_down.is_hovered(mouse_state.x, mouse_state.y) {
        bpm -= 1.0;
        click_result = ClickResult::ChangeBpm(bpm);
    }

    // play / pauses button
    let play_button = Rectangle {
        x: PLAY_X_ORIGIN,
        y: PLAY_Y_ORIGIN,
        width: PLAY_SQUARE_WIDTH,
        height: PLAY_SQUARE_HEIGHT,
    };
    if mouse_state.left_clicked && play_button.is_hovered(mouse_state.x, mouse_state.y) {
        is_playing = !is_playing;
        click_result = ClickResult::TogglePlay;
    }
    vertices.extend(play_button.draw(&screen_config, play_button.hover_color(mouse_state.x, mouse_state.y)));

    // stop button
    let stop_button = Rectangle {
        x: PLAY_X_ORIGIN + 64.0,
        y: PLAY_Y_ORIGIN,
        width: PLAY_SQUARE_WIDTH,
        height: PLAY_SQUARE_HEIGHT,
    };
    if mouse_state.left_clicked && stop_button.is_hovered(mouse_state.x, mouse_state.y) && active_step != 0 {
        click_result = ClickResult::Stop;
    }
    vertices.extend(stop_button.draw(&screen_config, stop_button.hover_color(mouse_state.x, mouse_state.y)));

    let sequencer_toggle = Rectangle {
        x: PLAY_X_ORIGIN + 256.0,
        y: PLAY_Y_ORIGIN,
        width: PLAY_SQUARE_WIDTH,
        height: PLAY_SQUARE_HEIGHT,
    };
    vertices.extend(sequencer_toggle.draw(&screen_config, sequencer_toggle.hover_color(mouse_state.x, mouse_state.y)));
    if mouse_state.left_clicked && sequencer_toggle.is_hovered(mouse_state.x, mouse_state.y) {
        click_result = ClickResult::ToggleSequencerWindow;
    }

    let mixer_toggle = Rectangle {
        x: PLAY_X_ORIGIN + 256.0 + (BUTTON_GAP * 3.0),
        y: PLAY_Y_ORIGIN,
        width: PLAY_SQUARE_WIDTH,
        height: PLAY_SQUARE_HEIGHT,
    };
    vertices.extend(mixer_toggle.draw(&screen_config, mixer_toggle.hover_color(mouse_state.x, mouse_state.y)));
    if mouse_state.left_clicked && mixer_toggle.is_hovered(mouse_state.x, mouse_state.y) {
        click_result = ClickResult::ToggleMixerWindow;
    }

    let playlist_toggle = Rectangle {
        x: PLAY_X_ORIGIN + 256.0 + (BUTTON_GAP * 3.0) * 2.0,
        y: PLAY_Y_ORIGIN,
        width: PLAY_SQUARE_WIDTH,
        height: PLAY_SQUARE_HEIGHT,
    };
    vertices.extend(playlist_toggle.draw(&screen_config, playlist_toggle.hover_color(mouse_state.x, mouse_state.y)));
    if mouse_state.left_clicked && playlist_toggle.is_hovered(mouse_state.x, mouse_state.y) {
        click_result = ClickResult::TogglePlaylistWindow;
    }

    let load_project = Rectangle {
        x: screen_config.width as f32 - LOAD_PROJECT_ICON_OFFSET,
        y: TOOLBAR_MARGIN,
        width: ICON_WIDTH,
        height: ICON_HEIGHT,
    };
    if mouse_state.left_clicked && load_project.is_hovered(mouse_state.x, mouse_state.y) {
        click_result = ClickResult::ProjectFileDialog;
    }

    let load_instrument = Rectangle {
        x: screen_config.width as f32 - ADD_INSTRUMENT_ICON_OFFSET,
        y: TOOLBAR_MARGIN,
        width: ICON_WIDTH,
        height: ICON_HEIGHT,
    };
    if mouse_state.left_clicked && load_instrument.is_hovered(mouse_state.x, mouse_state.y) {
        click_result = ClickResult::InstrumentFileDialog;
    }

    //     // toolbar line
    for vert in draw_h_line(TOOLBAR_Y, TOOLBAR_THICKNESS, screen_config) {
        vertices.push(vert);
    }

    //  bpm up button
    for vert in draw_rectangle(48.0, 4.0, 32.0, 10.0, screen_config, LIGHT_GRAY) {
        vertices.push(vert);
    }

    // bpm down button
    for vert in draw_rectangle(48.0, 16.0, 32.0, 10.0, screen_config, LIGHT_GRAY) {
        vertices.push(vert);
    }

    //     // play or pause
    let play_button = Rectangle {
        x: PLAY_X_ORIGIN as f32,
        y: PLAY_Y_ORIGIN as f32,
        width: PLAY_SQUARE_WIDTH as f32,
        height: PLAY_SQUARE_HEIGHT as f32,
    };
    vertices.extend(play_button.draw(screen_config, play_button.hover_color(mouse_state.x, mouse_state.y)));

    // load a file
    let load_file_button = Rectangle {
        x: screen_config.width as f32 - LOAD_PROJECT_ICON_OFFSET,
        y: TOOLBAR_MARGIN,
        width: ICON_WIDTH,
        height: ICON_HEIGHT,
    };
    vertices.extend(load_file_button.draw(screen_config, load_file_button.hover_color(mouse_state.x, mouse_state.y)));

    // load an instrument
    let instrument_button = Rectangle {
        x: screen_config.width as f32 - ADD_INSTRUMENT_ICON_OFFSET,
        y: TOOLBAR_MARGIN as f32,
        width: ICON_WIDTH as f32,
        height: ICON_HEIGHT as f32,
    };
    vertices.extend(instrument_button.draw(screen_config, instrument_button.hover_color(mouse_state.x, mouse_state.y)));

    let mut toolbar_texts: Vec<TextItem> = Vec::new();
    toolbar_texts.push(TextItem {
        text: "Patterns".to_string(),
        y: screen_config.width as f32 - 128.0 + PAD_4,
        x: TOOLBAR_Y + PAD_4,
    });

    for (i, pattern) in patterns.iter().enumerate() {
        toolbar_texts.push(TextItem {
            text: pattern.name.to_string(),
            x: screen_config.width as f32 - 96.0,
            y: 48.0 + (32.0 * i as f32) + 24.0,
        });
    }
    toolbar_texts.push(TextItem {
        text: "stop".to_string(),
        x: PLAY_X_ORIGIN + 64.0 + (PLAY_SQUARE_WIDTH / 4.0),
        y: 5.0,
    });
    toolbar_texts.push(TextItem {
        text: "sequence".to_string(),
        x: PLAY_X_ORIGIN + 256.0,
        y: 4.0,
    });
    toolbar_texts.push(TextItem {
        text: "mixer".to_string(),
        x: PLAY_X_ORIGIN + 256.0 + (BUTTON_GAP * 3.0),
        y: 4.0,
    });
    toolbar_texts.push(TextItem {
        text: "pl".to_string(),
        x: PLAY_X_ORIGIN + 256.0 + (BUTTON_GAP * 3.0) * 2.0,
        y: 4.0,
    });
    toolbar_texts.push(TextItem {
        text: "proj".to_string(),
        x: screen_config.width as f32 - 37.0,
        y: 4.0,
    });
    toolbar_texts.push(TextItem {
        text: "instr".to_string(),
        x: screen_config.width as f32 - (37.0 + 40.0 + 1.0),
        y: 4.0,
    });
    toolbar_texts.push(TextItem {
        text: bpm.to_string(),
        x: 10.0,
        y: TOOLBAR_MARGIN,
    });
    let label = if is_playing { "pause" } else { "play" };
    toolbar_texts.push(TextItem {
        text: label.to_string(),
        x: PLAY_X_ORIGIN + (PLAY_SQUARE_WIDTH / 4.0),
        y: 5.0,
    });
    toolbar_texts.push(TextItem {
        text: "Patterns".to_string(),
        y: screen_config.width as f32 - 128.0 + PAD_4,
        x: TOOLBAR_Y + PAD_4,
    });

    for (i, pattern) in patterns.iter().enumerate() {
        toolbar_texts.push(TextItem {
            text: pattern.name.to_string(),
            x: screen_config.width as f32 - 96.0,
            y: 48.0 + (32.0 * i as f32) + 24.0,
        });
    }
    toolbar_texts.push(TextItem {
        text: "stop".to_string(),
        x: PLAY_X_ORIGIN + 64.0 + (PLAY_SQUARE_WIDTH / 4.0),
        y: 5.0,
    });
    toolbar_texts.push(TextItem {
        text: "sequence".to_string(),
        x: PLAY_X_ORIGIN + 256.0,
        y: 4.0,
    });
    toolbar_texts.push(TextItem {
        text: "mixer".to_string(),
        x: PLAY_X_ORIGIN + 256.0 + (BUTTON_GAP * 3.0),
        y: 4.0,
    });
    toolbar_texts.push(TextItem {
        text: "pl".to_string(),
        x: PLAY_X_ORIGIN + 256.0 + (BUTTON_GAP * 3.0) * 2.0,
        y: 4.0,
    });
    toolbar_texts.push(TextItem {
        text: "proj".to_string(),
        x: screen_config.width as f32 - 37.0,
        y: 4.0,
    });
    toolbar_texts.push(TextItem {
        text: "instr".to_string(),
        x: screen_config.width as f32 - (37.0 + 40.0 + 1.0),
        y: 4.0,
    });
    toolbar_texts.push(TextItem {
        text: bpm.to_string(),
        x: 10.0,
        y: TOOLBAR_MARGIN,
    });
    let label = if is_playing { "pause" } else { "play" };
    toolbar_texts.push(TextItem {
        text: label.to_string(),
        x: PLAY_X_ORIGIN + (PLAY_SQUARE_WIDTH / 4.0),
        y: 5.0,
    });
    (vertices, toolbar_texts, click_result)
}
