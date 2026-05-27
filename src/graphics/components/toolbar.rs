use crate::app::MouseState;
use crate::graphics::{
    color::*,
    font::{TextItem, ROBOTO, TITLE},
    icons::{IconDraw, Tooltip},
    primitives::{draw_h_line, NO_RADIUS, PAD_32, PAD_8},
    widgets::*,
    ClickResult, ScreenConfig, Vertex, TOOLBAR_THICKNESS,
};
use winit::window::CursorIcon;

pub const TOOLTIP_MARGIN: f32 = 36.0;
pub const TOOLTIP_RIGHT_MARGIN: f32 = 96.0;
const LOAD_PROJECT_ICON_OFFSET: f32 = 40.0;
const WINDOW_ICONS_OFFSET: f32 = 256.0;
const ICON_GAP: f32 = 48.0;

pub fn icon_color(rect: &Square, mx: f32, my: f32, held: bool) -> Color {
    if rect.is_hovered(mx, my) && !held {
        DARK_GRAY_HOVER
    } else {
        DARK_GRAY
    }
}

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
    vertices.extend(toolbar_background.draw(&screen_config, PEBBLE, NO_RADIUS));

    // bpm button increment
    let bpm_up = Rectangle {
        x: 48.0,
        y: 4.0,
        width: 32.0,
        height: 10.0,
    };
    vertices.extend(bpm_up.draw(&screen_config, LIGHT_GRAY, NO_RADIUS));
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
    vertices.extend(bpm_down.draw(&screen_config, LIGHT_GRAY, NO_RADIUS));
    if bpm_down.is_hovered(mouse_state.x, mouse_state.y) {
        cursor_icon = CursorIcon::Pointer;
        if mouse_state.left_clicked {
            click_result = ClickResult::ChangeBpmDown;
        }
    }

    /* TRANSPORT CONTROL

        PLAY - start the song at the current step
        PAUSE - pause the song at the current step
        STOP - pause the song, reset current step
    */

    // play / pauses button
    let play_button = Square {
        x: PLAY_X_ORIGIN,
        y: PLAY_Y_ORIGIN,
        size: ICON_SIZE,
    };
    if play_button.is_hovered(mouse_state.x, mouse_state.y) {
        if mouse_state.left_clicked {
            click_result = ClickResult::TogglePlay;
        }
    }
    vertices.extend(play_button.draw(
        &screen_config,
        icon_color(&play_button, mouse_state.x, mouse_state.y, mouse_state.left_click_held),
        NO_RADIUS,
    ));

    // stop button
    let stop_button = Square {
        x: PLAY_X_ORIGIN + ICON_GAP,
        y: PLAY_Y_ORIGIN,
        size: ICON_SIZE,
    };
    if stop_button.is_hovered(mouse_state.x, mouse_state.y) {
        if mouse_state.left_clicked && active_step != 0 {
            click_result = ClickResult::Stop;
        }
    }

    vertices.extend(stop_button.draw(
        &screen_config,
        icon_color(&stop_button, mouse_state.x, mouse_state.y, mouse_state.left_click_held),
        NO_RADIUS,
    ));

    /* AUDIO TIME

    */

    let time_background = Rectangle {
        x: PLAY_X_ORIGIN + ICON_GAP + ICON_GAP,
        y: PLAY_Y_ORIGIN,
        width: ICON_SIZE * 4.0,
        height: ICON_SIZE,
    };
    vertices.extend(time_background.draw(&screen_config, BLACK, NO_RADIUS));

    /* MINI WINDOW TOGGLING
     *
     *  Sequencer
     *  Mixer
     *  Playlist
     *  Piano Roll
     *  Pattern Tray
     *  Track Tray
     */

    let sequencer_toggle = Square {
        x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET,
        y: PLAY_Y_ORIGIN,
        size: ICON_SIZE,
    };

    vertices.extend(sequencer_toggle.draw(
        &screen_config,
        icon_color(&sequencer_toggle, mouse_state.x, mouse_state.y, mouse_state.left_click_held),
        NO_RADIUS,
    ));
    if sequencer_toggle.is_hovered(mouse_state.x, mouse_state.y) {
        if mouse_state.left_clicked {
            click_result = ClickResult::ToggleSequencerWindow;
        }
    }

    let mixer_toggle = Square {
        x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET + ICON_GAP,
        y: PLAY_Y_ORIGIN,

        size: ICON_SIZE,
    };

    vertices.extend(mixer_toggle.draw(
        &screen_config,
        icon_color(&mixer_toggle, mouse_state.x, mouse_state.y, mouse_state.left_click_held),
        NO_RADIUS,
    ));
    if mixer_toggle.is_hovered(mouse_state.x, mouse_state.y) {
        if mouse_state.left_clicked {
            click_result = ClickResult::ToggleMixerWindow;
        }
    }

    let playlist_toggle = Square {
        x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET + (ICON_GAP * 2.0),
        y: PLAY_Y_ORIGIN,

        size: ICON_SIZE,
    };

    vertices.extend(playlist_toggle.draw(
        &screen_config,
        icon_color(&playlist_toggle, mouse_state.x, mouse_state.y, mouse_state.left_click_held),
        NO_RADIUS,
    ));
    if playlist_toggle.is_hovered(mouse_state.x, mouse_state.y) {
        if mouse_state.left_clicked {
            click_result = ClickResult::TogglePlaylistWindow;
        }
    }

    let piano_toggle = Square {
        x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET + (ICON_GAP * 3.0),
        y: PLAY_Y_ORIGIN,

        size: ICON_SIZE,
    };
    vertices.extend(piano_toggle.draw(
        &screen_config,
        icon_color(&piano_toggle, mouse_state.x, mouse_state.y, mouse_state.left_click_held),
        NO_RADIUS,
    ));
    if piano_toggle.is_hovered(mouse_state.x, mouse_state.y) {
        if mouse_state.left_clicked {
            click_result = ClickResult::TogglePianoRollWindow;
        }
    }

    let track_selection_toggle = Square {
        x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET + (ICON_GAP * 4.0),
        y: PLAY_Y_ORIGIN,

        size: ICON_SIZE,
    };
    vertices.extend(track_selection_toggle.draw(
        &screen_config,
        icon_color(&track_selection_toggle, mouse_state.x, mouse_state.y, mouse_state.left_click_held),
        NO_RADIUS,
    ));
    if track_selection_toggle.is_hovered(mouse_state.x, mouse_state.y) {
        if mouse_state.left_clicked {
            click_result = ClickResult::ToggleTrackTray;
        }
    }

    let patterns_toggle = Square {
        x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET + (ICON_GAP * 5.0),
        y: PLAY_Y_ORIGIN,

        size: ICON_SIZE,
    };
    vertices.extend(patterns_toggle.draw(
        &screen_config,
        icon_color(&patterns_toggle, mouse_state.x, mouse_state.y, mouse_state.left_click_held),
        NO_RADIUS,
    ));
    if patterns_toggle.is_hovered(mouse_state.x, mouse_state.y) {
        if mouse_state.left_clicked {
            click_result = ClickResult::TogglePatternTray;
        }
    }

    // toolbar line
    for vert in draw_h_line(TOOLBAR_Y, TOOLBAR_THICKNESS, screen_config) {
        vertices.push(vert);
    }

    /*  Project Composition I/O
     *
     * Load Project
     * Load Track
     */
    let load_project_button = Square {
        x: screen_config.width as f32 - LOAD_PROJECT_ICON_OFFSET,
        y: TOOLBAR_MARGIN,
        size: ICON_SIZE,
    };
    vertices.extend(load_project_button.draw(
        screen_config,
        icon_color(&load_project_button, mouse_state.x, mouse_state.y, mouse_state.left_click_held),
        NO_RADIUS,
    ));
    if load_project_button.is_hovered(mouse_state.x, mouse_state.y) {
        if mouse_state.left_clicked {
            click_result = ClickResult::ProjectFileDialog
        }
    }

    // load an track
    let load_track_button = Square {
        x: screen_config.width as f32 - ADD_TRACK_ICON_OFFSET,
        y: TOOLBAR_MARGIN,
        size: ICON_SIZE,
    };
    vertices.extend(load_track_button.draw(
        screen_config,
        icon_color(&load_track_button, mouse_state.x, mouse_state.y, mouse_state.left_click_held),
        NO_RADIUS,
    ));
    if load_track_button.is_hovered(mouse_state.x, mouse_state.y) {
        if mouse_state.left_clicked {
            click_result = ClickResult::TrackFileDialog
        }
    }

    toolbar_texts.push(TextItem {
        text: bpm.to_string(),
        x: PAD_8,
        y: TOOLBAR_MARGIN,
        size: TITLE,
        color: WHITE,
        font: ROBOTO,
    });

    let play_pause_label = if is_playing { "pause" } else { "play" };

    let icons = vec![
        IconDraw {
            name: "track",
            x: screen_config.width as f32 - ADD_TRACK_ICON_OFFSET,
            y: TOOLBAR_MARGIN,
            width: ICON_SIZE,
            height: ICON_SIZE,
            tooltip: Tooltip {
                text: Some("Add Track"),
                x: screen_config.width as f32 - ADD_TRACK_ICON_OFFSET - TOOLTIP_RIGHT_MARGIN,
                y: TOOLBAR_MARGIN + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: "project",
            x: screen_config.width as f32 - PAD_32 - PAD_8,
            y: TOOLBAR_MARGIN,
            width: ICON_SIZE,
            height: ICON_SIZE,
            tooltip: Tooltip {
                text: Some("Open Project"),
                x: screen_config.width as f32 - PAD_32 - PAD_8 - TOOLTIP_RIGHT_MARGIN,
                y: TOOLBAR_MARGIN + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: "sequencer",
            x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET,
            y: PLAY_Y_ORIGIN,
            width: ICON_SIZE,
            height: ICON_SIZE,
            tooltip: Tooltip {
                text: Some("Sequencer"),
                x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET,
                y: PLAY_Y_ORIGIN + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: "mixer",
            x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET + ICON_GAP,
            y: PLAY_Y_ORIGIN,
            width: ICON_SIZE,
            height: ICON_SIZE,
            tooltip: Tooltip {
                text: Some("Mixer"),
                x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET + ICON_GAP,
                y: PLAY_Y_ORIGIN + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: "playlist",
            x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET + ICON_GAP * 2.0,
            y: PLAY_Y_ORIGIN,
            width: ICON_SIZE,
            height: ICON_SIZE,
            tooltip: Tooltip {
                text: Some("Playlist"),
                x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET + ICON_GAP * 2.0,
                y: PLAY_Y_ORIGIN + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: "piano",
            x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET + ICON_GAP * 3.0,
            y: PLAY_Y_ORIGIN,
            width: ICON_SIZE,
            height: ICON_SIZE,
            tooltip: Tooltip {
                text: Some("Piano Roll"),
                x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET + ICON_GAP * 3.0,
                y: PLAY_Y_ORIGIN + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: play_pause_label,
            x: PLAY_X_ORIGIN,
            y: PLAY_Y_ORIGIN,
            width: ICON_SIZE,
            height: ICON_SIZE,
            tooltip: Tooltip {
                text: Some(if is_playing { "Pause" } else { "Play" }),
                x: (PLAY_X_ORIGIN),
                y: (PLAY_Y_ORIGIN + TOOLTIP_MARGIN),
            },
        },
        IconDraw {
            name: "stop",
            x: PLAY_X_ORIGIN + ICON_GAP,
            y: PLAY_Y_ORIGIN,
            width: ICON_SIZE,
            height: ICON_SIZE,
            tooltip: Tooltip {
                text: Some("Stop"),
                x: (PLAY_X_ORIGIN + 64.0),
                y: (PLAY_Y_ORIGIN + TOOLTIP_MARGIN),
            },
        },
        IconDraw {
            name: "track_tray",
            x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET + (ICON_GAP * 4.0),
            y: PLAY_Y_ORIGIN,
            width: ICON_SIZE,
            height: ICON_SIZE,
            tooltip: Tooltip {
                text: Some("Tracks"),
                x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET + (ICON_GAP * 4.0),
                y: PLAY_Y_ORIGIN + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: "pattern_tray",
            x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET + (ICON_GAP * 5.0),
            y: PLAY_Y_ORIGIN,
            width: ICON_SIZE,
            height: ICON_SIZE,
            tooltip: Tooltip {
                text: Some("Patterns"),
                x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET + (ICON_GAP * 4.0),
                y: PLAY_Y_ORIGIN + TOOLTIP_MARGIN,
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

    // debug current step
    toolbar_texts.push(TextItem {
        text: active_step.to_string(),
        x: 680.0,
        y: TOOLBAR_MARGIN,
        size: TITLE,
        color: WHITE,
        font: ROBOTO,
    });

    (vertices, toolbar_texts, icons, click_result, cursor_icon, tooltip)
}
