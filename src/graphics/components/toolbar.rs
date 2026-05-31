use crate::app::MouseState;
use crate::graphics::{
    color::*,
    font::{TextItem, MONOSPACED, TITLE},
    icons::{IconDraw, Tooltip},
    primitives::*,
    widgets::*,
    ClickResult, ScreenConfig, Vertex, TOOLBAR_THICKNESS,
};
use winit::window::CursorIcon;

pub const TOOLTIP_MARGIN: f32 = 36.0;
pub const TOOLTIP_RIGHT_MARGIN: f32 = 96.0;
const WINDOW_ICONS_OFFSET: f32 = 320.0;
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
    seconds: String,
) -> (Vec<Vertex>, Vec<TextItem>, Vec<IconDraw>, ClickResult, CursorIcon, Option<Tooltip>) {
    // setup
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut text_items: Vec<TextItem> = Vec::new();
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
    vertices.extend(toolbar_background.draw(screen_config, PEBBLE, RADIUS_4));

    /* BPM Control
     * Up
     * Down
     */

    let bpm_counter = TextItem {
        text: bpm.to_string(),
        x: PAD_8,
        y: 10.0,
        size: TITLE,
        color: WHITE,
        font: MONOSPACED,
    };

    let color = |rect: &Rectangle, m: &MouseState| {
        if rect.is_hovered(m.x, m.y) && !m.left_click_held {
            DARK_GRAY_HOVER
        } else {
            DARK_GRAY
        }
    };

    // bpm button increment
    let bpm_up = Rectangle {
        x: bpm_counter.x + 40.0,
        y: 6.0,
        width: PAD_32,
        height: 12.0,
    };
    vertices.extend(bpm_up.draw(screen_config, color(&bpm_up, mouse_state), RADIUS_4));
    if bpm_up.is_hovered(mouse_state.x, mouse_state.y) {
        cursor_icon = CursorIcon::Pointer;
        if mouse_state.left_clicked {
            click_result = ClickResult::ChangeBpmUp;
        }
    }
    // bpm button decrement
    let bpm_down = Rectangle {
        x: bpm_up.x,
        y: bpm_up.y + 18.0,
        width: bpm_up.width,
        height: bpm_up.height,
    };
    vertices.extend(bpm_down.draw(screen_config, color(&bpm_down, mouse_state), RADIUS_4));
    if bpm_down.is_hovered(mouse_state.x, mouse_state.y) {
        cursor_icon = CursorIcon::Pointer;
        if mouse_state.left_clicked {
            click_result = ClickResult::ChangeBpmDown;
        }
    }
    text_items.push(bpm_counter);

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
    if play_button.is_hovered(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
        click_result = ClickResult::TogglePlay;
    };
    vertices.extend(play_button.draw(
        screen_config,
        icon_color(&play_button, mouse_state.x, mouse_state.y, mouse_state.left_click_held),
        RADIUS_4,
    ));

    // stop button
    let stop_button = Square {
        x: PLAY_X_ORIGIN + ICON_GAP,
        y: PLAY_Y_ORIGIN,
        size: ICON_SIZE,
    };
    if stop_button.is_hovered(mouse_state.x, mouse_state.y) && mouse_state.left_clicked && active_step != 0 {
        click_result = ClickResult::Stop;
    };

    vertices.extend(stop_button.draw(
        screen_config,
        icon_color(&stop_button, mouse_state.x, mouse_state.y, mouse_state.left_click_held),
        RADIUS_4,
    ));

    /* AUDIO TIME

    */

    let time_background = Rectangle {
        x: PLAY_X_ORIGIN + ICON_GAP + ICON_GAP,
        y: PLAY_Y_ORIGIN,
        width: ICON_SIZE * 5.0,
        height: ICON_SIZE,
    };
    vertices.extend(time_background.draw(screen_config, BLACK, RADIUS_4));

    let step_label = if active_step < 10 {
        format!("0{}", active_step)
    } else {
        active_step.to_string()
    };

    // debug current step
    text_items.push(TextItem {
        text: step_label,
        x: time_background.x + time_background.width - PAD_16 - PAD_8 - PAD_4 - PAD_2,
        y: TOOLBAR_MARGIN + PAD_2,
        size: TITLE,
        color: ORANGE,
        font: MONOSPACED,
    });
    text_items.push(TextItem {
        text: seconds,
        x: time_background.x + time_background.width - PAD_32 * 5.0 + PAD_4,
        y: TOOLBAR_MARGIN + PAD_2,
        size: TITLE,
        color: ORANGE,
        font: MONOSPACED,
    });

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
        screen_config,
        icon_color(&sequencer_toggle, mouse_state.x, mouse_state.y, mouse_state.left_click_held),
        RADIUS_4,
    ));
    if sequencer_toggle.is_hovered(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
        click_result = ClickResult::ToggleSequencerWindow;
    };

    let mixer_toggle = Square {
        x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET + ICON_GAP,
        y: PLAY_Y_ORIGIN,

        size: ICON_SIZE,
    };

    vertices.extend(mixer_toggle.draw(
        screen_config,
        icon_color(&mixer_toggle, mouse_state.x, mouse_state.y, mouse_state.left_click_held),
        RADIUS_4,
    ));
    if mixer_toggle.is_hovered(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
        click_result = ClickResult::ToggleMixerWindow;
    };

    let playlist_toggle = Square {
        x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET + (ICON_GAP * 2.0),
        y: PLAY_Y_ORIGIN,

        size: ICON_SIZE,
    };

    vertices.extend(playlist_toggle.draw(
        screen_config,
        icon_color(&playlist_toggle, mouse_state.x, mouse_state.y, mouse_state.left_click_held),
        RADIUS_4,
    ));
    if playlist_toggle.is_hovered(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
        click_result = ClickResult::TogglePlaylistWindow;
    };

    let piano_toggle = Square {
        x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET + (ICON_GAP * 3.0),
        y: PLAY_Y_ORIGIN,

        size: ICON_SIZE,
    };
    vertices.extend(piano_toggle.draw(
        screen_config,
        icon_color(&piano_toggle, mouse_state.x, mouse_state.y, mouse_state.left_click_held),
        RADIUS_4,
    ));
    if piano_toggle.is_hovered(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
        click_result = ClickResult::TogglePianoRollWindow;
    };

    let track_selection_toggle = Square {
        x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET + (ICON_GAP * 4.0),
        y: PLAY_Y_ORIGIN,

        size: ICON_SIZE,
    };
    vertices.extend(track_selection_toggle.draw(
        screen_config,
        icon_color(&track_selection_toggle, mouse_state.x, mouse_state.y, mouse_state.left_click_held),
        RADIUS_4,
    ));
    if track_selection_toggle.is_hovered(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
        click_result = ClickResult::ToggleTrackTray;
    };

    let patterns_toggle = Square {
        x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET + (ICON_GAP * 5.0),
        y: PLAY_Y_ORIGIN,

        size: ICON_SIZE,
    };
    vertices.extend(patterns_toggle.draw(
        screen_config,
        icon_color(&patterns_toggle, mouse_state.x, mouse_state.y, mouse_state.left_click_held),
        RADIUS_4,
    ));
    if patterns_toggle.is_hovered(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
        click_result = ClickResult::TogglePatternTray;
    };

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
        x: screen_config.width as f32 - 40.0,
        y: TOOLBAR_MARGIN,
        size: ICON_SIZE,
    };
    vertices.extend(load_project_button.draw(
        screen_config,
        icon_color(&load_project_button, mouse_state.x, mouse_state.y, mouse_state.left_click_held),
        RADIUS_4,
    ));
    if load_project_button.is_hovered(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
        click_result = ClickResult::ProjectFileDialog
    };

    // load an track
    let load_track_button = Square {
        x: load_project_button.x - ICON_GAP,
        y: TOOLBAR_MARGIN,
        size: ICON_SIZE,
    };
    vertices.extend(load_track_button.draw(
        screen_config,
        icon_color(&load_track_button, mouse_state.x, mouse_state.y, mouse_state.left_click_held),
        RADIUS_4,
    ));
    if load_track_button.is_hovered(mouse_state.x, mouse_state.y) && mouse_state.left_clicked {
        click_result = ClickResult::TrackFileDialog
    };

    let icons = vec![
        IconDraw {
            name: "track",
            x: load_track_button.x,
            y: load_track_button.y,
            width: ICON_SIZE,
            height: ICON_SIZE,
            tooltip: Tooltip {
                text: Some("Add Track"),
                x: load_project_button.x - ICON_GAP - TOOLTIP_RIGHT_MARGIN,
                y: load_track_button.y + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: "project",
            x: load_project_button.x,
            y: load_project_button.y,
            width: ICON_SIZE,
            height: ICON_SIZE,
            tooltip: Tooltip {
                text: Some("Open Project"),
                x: load_project_button.x - TOOLTIP_RIGHT_MARGIN,
                y: load_project_button.y + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: "sequencer",
            x: sequencer_toggle.x,
            y: sequencer_toggle.y,
            width: ICON_SIZE,
            height: ICON_SIZE,
            tooltip: Tooltip {
                text: Some("Sequencer"),
                x: sequencer_toggle.x,
                y: sequencer_toggle.y + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: "mixer",
            x: mixer_toggle.x,
            y: mixer_toggle.y,
            width: ICON_SIZE,
            height: ICON_SIZE,
            tooltip: Tooltip {
                text: Some("Mixer"),
                x: mixer_toggle.x,
                y: mixer_toggle.y + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: "playlist",
            x: playlist_toggle.x,
            y: playlist_toggle.y,
            width: ICON_SIZE,
            height: ICON_SIZE,
            tooltip: Tooltip {
                text: Some("Playlist"),
                x: playlist_toggle.x,
                y: playlist_toggle.y + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: "piano",
            x: piano_toggle.x,
            y: piano_toggle.y,
            width: ICON_SIZE,
            height: ICON_SIZE,
            tooltip: Tooltip {
                text: Some("Piano Roll"),
                x: piano_toggle.x,
                y: piano_toggle.y + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: if is_playing { "pause" } else { "play" },
            x: play_button.x,
            y: play_button.y,
            width: ICON_SIZE,
            height: ICON_SIZE,
            tooltip: Tooltip {
                text: Some(if is_playing { "Pause" } else { "Play" }),
                x: play_button.x,
                y: play_button.y + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: "stop",
            x: stop_button.x,
            y: stop_button.y,
            width: ICON_SIZE,
            height: ICON_SIZE,
            tooltip: Tooltip {
                text: Some("Stop"),
                x: stop_button.x,
                y: stop_button.y + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: "track_tray",
            x: track_selection_toggle.x,
            y: track_selection_toggle.y,
            width: ICON_SIZE,
            height: ICON_SIZE,
            tooltip: Tooltip {
                text: Some("Tracks"),
                x: track_selection_toggle.x,
                y: track_selection_toggle.y + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: "pattern_tray",
            x: patterns_toggle.x,
            y: patterns_toggle.y,
            width: ICON_SIZE,
            height: ICON_SIZE,
            tooltip: Tooltip {
                text: Some("Patterns"),
                x: patterns_toggle.x,
                y: patterns_toggle.y + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: "bpm_up",
            x: bpm_up.x,
            y: bpm_up.y,
            width: bpm_up.width,
            height: bpm_up.height,
            tooltip: Tooltip {
                text: Some("Increment BPM"),
                x: bpm_up.x,
                y: bpm_up.y + TOOLTIP_MARGIN,
            },
        },
        IconDraw {
            name: "bpm_down",
            x: bpm_up.x,
            y: bpm_up.y + 18.0,
            width: bpm_up.width,
            height: bpm_up.height,
            tooltip: Tooltip {
                text: Some("Decrement BPM"),
                x: bpm_up.x,
                y: bpm_up.y + TOOLTIP_MARGIN,
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

    let step_divider_line = Rectangle {
        x: time_background.x + time_background.width - PAD_16 - PAD_8 - PAD_16,
        y: PAD_4,
        height: TOOLBAR_Y - PAD_8 - PAD_2,
        width: 2.0,
    };
    vertices.extend(step_divider_line.draw(screen_config, DARK_GRAY, NO_RADIUS));

    (vertices, text_items, icons, click_result, cursor_icon, tooltip)
}
