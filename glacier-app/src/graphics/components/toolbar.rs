//! The toolbar contains easy access toggle buttons for opening windows, loading files, or view project bpm/step/time info.
use crate::app::MouseState;
use crate::graphics::components::spectrum;
use crate::graphics::{
    color::*,
    font::{TextItem, MONOSPACED, TITLE},
    geometry::*,
    icons::{IconDraw, Tooltip},
    primitives::*,
    ClickResult, ScreenConfig, Vertex,
};
use winit::window::CursorIcon;

pub const TOOLBAR_Y: f32 = 42.0;
pub const TOOLBAR_THICKNESS: f32 = 0.003;
pub const TOOLBAR_MARGIN: f32 = 4.0;

pub const TOOLTIP_MARGIN: f32 = 36.0;
pub const TOOLTIP_RIGHT_MARGIN: f32 = 96.0;

const WINDOW_ICONS_OFFSET: f32 = 320.0;
const ICON_GAP: f32 = 48.0;
pub const ICON_SIZE: f32 = 32.0;

pub const PLAY_Y_ORIGIN: f32 = 4.0;
pub const PLAY_X_ORIGIN: f32 = 90.0;

struct IconRow {
    x: f32,
    y: f32,
}
impl IconRow {
    fn next(&mut self) -> Square {
        let sq = Square {
            x: self.x,
            y: self.y,
            size: ICON_SIZE,
        };
        self.x += ICON_GAP;
        sq
    }
}

pub fn draw_toolbar(
    mouse_state: &MouseState,
    screen_config: &ScreenConfig,
    bpm: f32,
    is_playing: bool,
    active_step: usize,
    spectrum: &Vec<f32>,
    sample_rate: f32,
    seconds: String,
    out: &mut Vec<Vertex>,
) -> (
    Vec<TextItem>,
    Vec<IconDraw>,
    ClickResult,
    CursorIcon,
    Option<Tooltip>,
) {
    let mut text_items: Vec<TextItem> = Vec::new();
    let mut click_result = ClickResult::None;
    let mut cursor_icon = CursorIcon::Default;
    let mut tooltip: Option<Tooltip> = None;

    let mut window_icons = IconRow {
        x: PLAY_X_ORIGIN + WINDOW_ICONS_OFFSET,
        y: PLAY_Y_ORIGIN,
    };

    let toolbar_background = Rectangle {
        x: 0.0,
        y: 0.0,
        width: screen_config.width as f32,
        height: TOOLBAR_Y,
    };
    toolbar_background.draw(screen_config, SURFACE, NO_RADIUS, out);

    let toolbar_divider = Rectangle {
        x: toolbar_background.x,
        y: toolbar_background.y + toolbar_background.height,
        height: 1.0,
        width: toolbar_background.width,
    };
    toolbar_divider.draw(screen_config, DARK_GRAY, NO_RADIUS, out);

    let bpm_counter = TextItem {
        text: bpm.to_string(),
        x: PAD_8,
        y: 10.0,
        size: TITLE,
        color: WHITE,
        font: MONOSPACED,
    };

    let bpm_up = Rectangle {
        x: bpm_counter.x + 40.0,
        y: 6.0,
        width: PAD_32,
        height: 12.0,
    };
    let bpm_up_hovered =
        bpm_up.draw_interactive(screen_config, DARK_GRAY, mouse_state, RADIUS_4, out);
    if bpm_up_hovered {
        cursor_icon = CursorIcon::Pointer;
        if mouse_state.left_clicked {
            click_result = ClickResult::ChangeBpmUp;
        }
    }

    let bpm_down = Rectangle {
        x: bpm_up.x,
        y: bpm_up.y + 18.0,
        width: bpm_up.width,
        height: bpm_up.height,
    };
    let bpm_down_hovered =
        bpm_down.draw_interactive(screen_config, DARK_GRAY, mouse_state, RADIUS_4, out);
    if bpm_down_hovered {
        cursor_icon = CursorIcon::Pointer;
        if mouse_state.left_clicked {
            click_result = ClickResult::ChangeBpmDown;
        }
    }
    text_items.push(bpm_counter);

    let play_button = Square {
        x: PLAY_X_ORIGIN,
        y: PLAY_Y_ORIGIN,
        size: ICON_SIZE,
    };
    let play_hovered =
        play_button.draw_interactive(screen_config, DARK_GRAY, mouse_state, RADIUS_4, out);
    if play_hovered && mouse_state.left_clicked {
        click_result = ClickResult::TogglePlay;
    }

    let stop_button = Square {
        x: PLAY_X_ORIGIN + ICON_GAP,
        y: PLAY_Y_ORIGIN,
        size: ICON_SIZE,
    };
    let stop_hovered =
        stop_button.draw_interactive(screen_config, DARK_GRAY, mouse_state, RADIUS_4, out);
    if stop_hovered && mouse_state.left_clicked && active_step != 0 {
        click_result = ClickResult::Stop;
    }

    let time_background = Rectangle {
        x: PLAY_X_ORIGIN + ICON_GAP + ICON_GAP,
        y: PLAY_Y_ORIGIN,
        width: ICON_SIZE * 5.0,
        height: ICON_SIZE,
    };
    time_background.draw(screen_config, BLACK, RADIUS_4, out);

    let step_label = if active_step < 10 {
        format!("0{}", active_step)
    } else {
        active_step.to_string()
    };

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

    // draw power spectrogram of audio frequency domain.
    spectrum::draw(screen_config, &spectrum, sample_rate, 2048, out);

    let sequencer_toggle = window_icons.next();
    let sequencer_hovered =
        sequencer_toggle.draw_interactive(screen_config, DARK_GRAY, mouse_state, RADIUS_4, out);
    if sequencer_hovered && mouse_state.left_clicked {
        click_result = ClickResult::ToggleSequencerWindow;
    }

    let mixer_toggle = window_icons.next();
    let mixer_hovered =
        mixer_toggle.draw_interactive(screen_config, DARK_GRAY, mouse_state, RADIUS_4, out);
    if mixer_hovered && mouse_state.left_clicked {
        click_result = ClickResult::ToggleMixerWindow;
    }

    let playlist_toggle = window_icons.next();
    let playlist_hovered =
        playlist_toggle.draw_interactive(screen_config, DARK_GRAY, mouse_state, RADIUS_4, out);
    if playlist_hovered && mouse_state.left_clicked {
        click_result = ClickResult::TogglePlaylistWindow;
    }

    let piano_toggle = window_icons.next();
    let piano_hovered =
        piano_toggle.draw_interactive(screen_config, DARK_GRAY, mouse_state, RADIUS_4, out);
    if piano_hovered && mouse_state.left_clicked {
        click_result = ClickResult::TogglePianoRollWindow;
    }

    let track_selection_toggle = window_icons.next();
    let track_selection_hovered = track_selection_toggle.draw_interactive(
        screen_config,
        DARK_GRAY,
        mouse_state,
        RADIUS_4,
        out,
    );
    if track_selection_hovered && mouse_state.left_clicked {
        click_result = ClickResult::ToggleTrackTray;
    }

    let patterns_toggle = window_icons.next();
    let patterns_hovered =
        patterns_toggle.draw_interactive(screen_config, DARK_GRAY, mouse_state, RADIUS_4, out);
    if patterns_hovered && mouse_state.left_clicked {
        click_result = ClickResult::TogglePatternTray;
    }

    draw_h_line(TOOLBAR_Y, TOOLBAR_THICKNESS, screen_config, out);

    let load_project_button = Square {
        x: screen_config.width as f32 - 40.0,
        y: TOOLBAR_MARGIN,
        size: ICON_SIZE,
    };
    let load_project_hovered =
        load_project_button.draw_interactive(screen_config, DARK_GRAY, mouse_state, RADIUS_4, out);
    if load_project_hovered && mouse_state.left_clicked {
        click_result = ClickResult::ProjectFileDialog;
    }

    let load_track_button = Square {
        x: load_project_button.x - ICON_GAP,
        y: TOOLBAR_MARGIN,
        size: ICON_SIZE,
    };
    let load_track_hovered =
        load_track_button.draw_interactive(screen_config, DARK_GRAY, mouse_state, RADIUS_4, out);
    if load_track_hovered && mouse_state.left_clicked {
        click_result = ClickResult::TrackFileDialog;
    }

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
    step_divider_line.draw(screen_config, DARK_GRAY, NO_RADIUS, out);

    (text_items, icons, click_result, cursor_icon, tooltip)
}
