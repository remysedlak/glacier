use crate::app::MouseState;
use crate::graphics::{
    color::{LIGHT_GRAY, PEBBLE, WHITE},
    font::TextItem,
    icons::{IconDraw, Tooltip},
    primitives::{draw_h_line, BUTTON_GAP, PAD_32, PAD_8},
    widgets::{Rectangle, ADD_INSTRUMENT_ICON_OFFSET, ICON_HEIGHT, ICON_WIDTH, PLAY_X_ORIGIN, PLAY_Y_ORIGIN, TOOLBAR_MARGIN, TOOLBAR_Y},
    ClickResult, ScreenConfig, Vertex, TOOLBAR_THICKNESS,
};
use winit::window::CursorIcon;

const LOAD_PROJECT_ICON_OFFSET: f32 = 40.0;

pub fn draw_toolbar(
    mouse_state: &MouseState,
    screen_config: &ScreenConfig,
    bpm: f32,
    is_playing: bool,
    active_step: usize,
) -> (Vec<Vertex>, Vec<TextItem>, Vec<IconDraw>, ClickResult, CursorIcon, Option<Tooltip>) {
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut toolbar_texts: Vec<TextItem> = Vec::new();
    let mut click_result = ClickResult::None;
    let mut cursor_icon = CursorIcon::Default;
    let mut tooltip: Option<Tooltip> = None;

    // background of toolbar's buttons and components
    let toolbar_background = Rectangle {
        x: 0.0,
        y: 0.0,
        width: screen_config.width as f32,
        height: TOOLBAR_Y,
    };
    vertices.extend(toolbar_background.draw(&screen_config, PEBBLE));

    // bpm button increment
    let bpm_up = Rectangle {
        x: 48.0,
        y: 4.0,
        width: 32.0,
        height: 10.0,
    };
    vertices.extend(bpm_up.draw(&screen_config, LIGHT_GRAY));
    if bpm_up.is_hovered(mouse_state.x, mouse_state.y) {
        cursor_icon = CursorIcon::Pointer;
        if mouse_state.left_clicked {
            click_result = ClickResult::ChangeBpmUp;
        }
    }
    // bpm button decrement
    let bpm_down = Rectangle {
        x: 48.0,
        y: 16.0,
        width: 32.0,
        height: 10.0,
    };
    vertices.extend(bpm_down.draw(&screen_config, LIGHT_GRAY));
    if bpm_down.is_hovered(mouse_state.x, mouse_state.y) {
        cursor_icon = CursorIcon::Pointer;
        if mouse_state.left_clicked {
            click_result = ClickResult::ChangeBpmDown;
        }
    }

    // play / pauses button
    let play_button = Rectangle {
        x: PLAY_X_ORIGIN,
        y: PLAY_Y_ORIGIN,
        width: ICON_WIDTH,
        height: ICON_HEIGHT,
    };
    if play_button.is_hovered(mouse_state.x, mouse_state.y) {
        if mouse_state.left_clicked {
            click_result = ClickResult::TogglePlay;
        }
    }
    vertices.extend(play_button.draw(
        &screen_config,
        play_button.dark_hover_color(mouse_state.x, mouse_state.y, mouse_state.left_clicked),
    ));

    // stop button
    let stop_button = Rectangle {
        x: PLAY_X_ORIGIN + 64.0,
        y: PLAY_Y_ORIGIN,
        width: ICON_WIDTH,
        height: ICON_HEIGHT,
    };
    if stop_button.is_hovered(mouse_state.x, mouse_state.y) {
        if mouse_state.left_clicked && active_step != 0 {
            click_result = ClickResult::Stop;
        }
    }
    vertices.extend(stop_button.draw(
        &screen_config,
        stop_button.dark_hover_color(mouse_state.x, mouse_state.y, mouse_state.left_clicked),
    ));

    let sequencer_toggle = Rectangle {
        x: PLAY_X_ORIGIN + 256.0,
        y: PLAY_Y_ORIGIN,
        width: ICON_WIDTH,
        height: ICON_HEIGHT,
    };
    vertices.extend(sequencer_toggle.draw(
        &screen_config,
        sequencer_toggle.dark_hover_color(mouse_state.x, mouse_state.y, mouse_state.left_clicked),
    ));
    if sequencer_toggle.is_hovered(mouse_state.x, mouse_state.y) {
        if mouse_state.left_clicked {
            click_result = ClickResult::ToggleSequencerWindow;
        }
    }

    let mixer_toggle = Rectangle {
        x: PLAY_X_ORIGIN + 256.0 + (BUTTON_GAP * 2.0),
        y: PLAY_Y_ORIGIN,
        width: ICON_WIDTH,
        height: ICON_HEIGHT,
    };
    vertices.extend(mixer_toggle.draw(
        &screen_config,
        mixer_toggle.dark_hover_color(mouse_state.x, mouse_state.y, mouse_state.left_clicked),
    ));
    if mixer_toggle.is_hovered(mouse_state.x, mouse_state.y) {
        if mouse_state.left_clicked {
            click_result = ClickResult::ToggleMixerWindow;
        }
    }

    let playlist_toggle = Rectangle {
        x: PLAY_X_ORIGIN + 256.0 + (BUTTON_GAP * 2.0) * 2.0,
        y: PLAY_Y_ORIGIN,
        width: ICON_WIDTH,
        height: ICON_HEIGHT,
    };
    vertices.extend(playlist_toggle.draw(
        &screen_config,
        playlist_toggle.dark_hover_color(mouse_state.x, mouse_state.y, mouse_state.left_clicked),
    ));
    if playlist_toggle.is_hovered(mouse_state.x, mouse_state.y) {
        if mouse_state.left_clicked {
            click_result = ClickResult::TogglePlaylistWindow;
        }
    }

    // toolbar line
    for vert in draw_h_line(TOOLBAR_Y, TOOLBAR_THICKNESS, screen_config) {
        vertices.push(vert);
    }

    // load a file
    let load_file_button = Rectangle {
        x: screen_config.width as f32 - LOAD_PROJECT_ICON_OFFSET,
        y: TOOLBAR_MARGIN,
        width: ICON_WIDTH,
        height: ICON_HEIGHT,
    };
    vertices.extend(load_file_button.draw(
        screen_config,
        load_file_button.dark_hover_color(mouse_state.x, mouse_state.y, mouse_state.left_clicked),
    ));
    if load_file_button.is_hovered(mouse_state.x, mouse_state.y) {
        if mouse_state.left_clicked {
            click_result = ClickResult::ProjectFileDialog
        }
    }

    // load an instrument
    let instrument_button = Rectangle {
        x: screen_config.width as f32 - ADD_INSTRUMENT_ICON_OFFSET,
        y: TOOLBAR_MARGIN as f32,
        width: ICON_WIDTH as f32,
        height: ICON_HEIGHT as f32,
    };
    vertices.extend(instrument_button.draw(
        screen_config,
        instrument_button.dark_hover_color(mouse_state.x, mouse_state.y, mouse_state.left_clicked),
    ));
    if instrument_button.is_hovered(mouse_state.x, mouse_state.y) {
        if mouse_state.left_clicked {
            click_result = ClickResult::InstrumentFileDialog
        }
    }

    toolbar_texts.push(TextItem {
        text: bpm.to_string(),
        x: 10.0,
        y: TOOLBAR_MARGIN,
        size: 18.0,
        color: WHITE,
        font: "roboto",
    });

    let play_pause_label = if is_playing { "pause" } else { "play" };

    let icon_size = 32.0;
    pub const TOOLTIP_MARGIN: f32 = 36.0;
    pub const TOOLTIP_RIGHT_MARGIN: f32 = 96.0;

    let icons = vec![
        IconDraw {
            name: "instrument",
            x: screen_config.width as f32 - ADD_INSTRUMENT_ICON_OFFSET,
            y: TOOLBAR_MARGIN,
            width: icon_size,
            height: icon_size,
            tooltip: Tooltip {
                text: Some("Add Instrument"),
                x: screen_config.width as f32 - ADD_INSTRUMENT_ICON_OFFSET - TOOLTIP_RIGHT_MARGIN,
                y: TOOLBAR_MARGIN + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: "project",
            x: screen_config.width as f32 - PAD_32 - PAD_8,
            y: TOOLBAR_MARGIN,
            width: icon_size,
            height: icon_size,
            tooltip: Tooltip {
                text: Some("Open Project"),
                x: screen_config.width as f32 - PAD_32 - PAD_8 - TOOLTIP_RIGHT_MARGIN,
                y: TOOLBAR_MARGIN + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: "sequencer",
            x: PLAY_X_ORIGIN + 256.0,
            y: PLAY_Y_ORIGIN,
            width: icon_size,
            height: icon_size,
            tooltip: Tooltip {
                text: Some("Open Sequencer"),
                x: PLAY_X_ORIGIN + 256.0,
                y: PLAY_Y_ORIGIN + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: "mixer",
            x: PLAY_X_ORIGIN + 256.0 + (BUTTON_GAP * 2.0),
            y: PLAY_Y_ORIGIN,
            width: icon_size,
            height: icon_size,
            tooltip: Tooltip {
                text: Some("Open Mixer"),
                x: PLAY_X_ORIGIN + 256.0 + (BUTTON_GAP * 2.0),
                y: PLAY_Y_ORIGIN + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: "playlist",
            x: PLAY_X_ORIGIN + 256.0 + (BUTTON_GAP * 2.0) * 2.0,
            y: PLAY_Y_ORIGIN,
            width: icon_size,
            height: icon_size,
            tooltip: Tooltip {
                text: Some("Open Playlist"),
                x: PLAY_X_ORIGIN + 256.0 + (BUTTON_GAP * 2.0) * 2.0,
                y: PLAY_Y_ORIGIN + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: play_pause_label,
            x: PLAY_X_ORIGIN,
            y: PLAY_Y_ORIGIN,
            width: icon_size,
            height: icon_size,
            tooltip: Tooltip {
                text: Some(if is_playing { "Pause" } else { "Play" }),
                x: (PLAY_X_ORIGIN),
                y: (PLAY_Y_ORIGIN + TOOLTIP_MARGIN),
            },
        },
        IconDraw {
            name: "stop",
            x: PLAY_X_ORIGIN + 64.0,
            y: PLAY_Y_ORIGIN,
            width: icon_size,
            height: icon_size,
            tooltip: Tooltip {
                text: Some("Stop"),
                x: (PLAY_X_ORIGIN + 64.0),
                y: (PLAY_Y_ORIGIN + TOOLTIP_MARGIN),
            },
        },
    ];

    if !mouse_state.left_click_held {
        for icon in &icons {
            if icon.is_hovered(mouse_state.x, mouse_state.y) {
                tooltip = Some(icon.tooltip.clone());
            }
        }
    }

    (vertices, toolbar_texts, icons, click_result, cursor_icon, tooltip)
}
