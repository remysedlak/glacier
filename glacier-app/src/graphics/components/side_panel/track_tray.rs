use crate::graphics::color::{DARK_GRAY, DARK_GRAY_HOVER, DARK_GRAY_HOVER_HOVER, LL_GRAY};
use crate::graphics::icons::IconDraw;
use crate::graphics::Tooltip;
use crate::project::Track;
use crate::{
    app::MouseState,
    graphics::{
        color::{LIGHT_GRAY, PEBBLE, WHITE},
        components::side_panel::{PATTERN_TRAY_HEADER_MARGIN, PATTERN_TRAY_ITEM_GAP},
        font::{truncate_text, TextItem, ROBOTO},
        primitives::*,
        side_panel::{draw_title, PATTERN_TRAY_ITEM_HEIGHT},
        widgets::{Rectangle, TOOLBAR_Y},
        ClickResult,
    },
};
use std::collections::HashMap;
use std::path::PathBuf;
use winit::window::CursorIcon;

pub fn draw(
    mouse_state: &MouseState,
    screen_config: &ScreenConfig,
    tracks: &[Track],
    user_fs_location: &std::path::Path,
    expanded_dirs: &std::collections::HashSet<PathBuf>,
    fs_cache: &HashMap<PathBuf, Vec<(PathBuf, bool)>>,
    tray_width: f32,
    out: &mut Vec<Vertex>,
) -> (Vec<TextItem>, Vec<IconDraw>, ClickResult, CursorIcon) {
    let mut text_items: Vec<TextItem> = Vec::new();
    let mut cursor_icon: CursorIcon = CursorIcon::Default;
    let mut click_result: ClickResult = ClickResult::None;

    let track_tray = Rectangle {
        x: 0.0,
        y: TOOLBAR_Y,
        width: tray_width,
        height: screen_config.height as f32 - TOOLBAR_Y,
    };
    track_tray.draw(screen_config, PEBBLE, NO_RADIUS, out);
    let w_divider = Rectangle {
        x: track_tray.x + track_tray.width,
        y: track_tray.y,
        width: 1.0,
        height: track_tray.height,
    };
    w_divider.draw(screen_config, LL_GRAY, NO_RADIUS, out);
    if track_tray.is_hovered_right_edge(mouse_state.x, mouse_state.y) {
        cursor_icon = CursorIcon::ColResize
    }

    text_items.push(draw_title("Tracks", (track_tray.x, track_tray.y)));

    for (i, track) in tracks.iter().enumerate() {
        let button_x = track_tray.x + PAD_2;
        let button_y = PATTERN_TRAY_HEADER_MARGIN + (PATTERN_TRAY_ITEM_GAP * i as f32) + PAD_32;
        let track_button = Rectangle {
            x: button_x,
            y: button_y,
            width: track_tray.width - PAD_4,
            height: PATTERN_TRAY_ITEM_HEIGHT,
        };

        let track_button_color = if track_button.is_hovered(mouse_state.x, mouse_state.y) {
            DARK_GRAY_HOVER_HOVER
        } else {
            DARK_GRAY_HOVER
        };

        track_button.draw(screen_config, track_button_color, RADIUS_4, out);
        if track_button.is_hovered(mouse_state.x, mouse_state.y) {
            cursor_icon = CursorIcon::Pointer;
            if mouse_state.left_double_clicked {
                click_result = ClickResult::ToggleTrackWindow(i);
            }
        }

        text_items.push(TextItem {
            text: truncate_text(&track.data.name, 18),
            x: track_button.x + PAD_4,
            y: PATTERN_TRAY_HEADER_MARGIN + (PATTERN_TRAY_ITEM_GAP * i as f32) + PAD_32 + PAD_2,
            size: 10.0,
            color: WHITE,
            font: ROBOTO,
        });
    }

    let w_divider = Rectangle {
        x: PAD_2,
        y: (screen_config.height / 2) as f32,
        width: tray_width - PAD_4,
        height: 1.0,
    };
    w_divider.draw(screen_config, LIGHT_GRAY, RADIUS_4, out);
    text_items.push(draw_title("File Tree", (w_divider.x - 2.0, w_divider.y)));

    let mut row: f32 = 0.0;
    let icons = draw_fs_tree(
        user_fs_location,
        0,
        &mut row,
        w_divider.y + PAD_32 + PAD_16,
        expanded_dirs,
        mouse_state,
        screen_config,
        &mut text_items,
        &mut click_result,
        &mut cursor_icon,
        fs_cache,
        tray_width,
        out,
    );

    (text_items, icons, click_result, cursor_icon)
}

fn draw_fs_tree(
    dir: &std::path::Path,
    depth: usize,
    row: &mut f32,
    base_y: f32,
    expanded_dirs: &std::collections::HashSet<PathBuf>,
    mouse_state: &MouseState,
    screen_config: &ScreenConfig,
    text_items: &mut Vec<TextItem>,
    click_result: &mut ClickResult,
    cursor_icon: &mut CursorIcon,
    fs_cache: &HashMap<PathBuf, Vec<(PathBuf, bool)>>,
    tray_width: f32,
    out: &mut Vec<Vertex>,
) -> Vec<IconDraw> {
    let mut icons: Vec<IconDraw> = Vec::new();
    let Some(entries) = fs_cache.get(dir) else {
        return Vec::new();
    };
    let indent = depth as f32 * PAD_16;

    for (path, is_dir) in entries {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");

        let y = base_y + *row * PAD_32;
        let button = Rectangle {
            height: 24.0,
            width: tray_width - PAD_4 * 2.0 - indent,
            x: PAD_4 + indent,
            y,
        };
        let color = if button.is_hovered(mouse_state.x, mouse_state.y) {
            DARK_GRAY
        } else {
            PEBBLE
        };
        button.draw(screen_config, color, RADIUS_4, out);

        text_items.push(TextItem {
            text: truncate_text(name, 25),
            x: button.x + PAD_4 + 16.0,
            y: button.y + PAD_4,
            size: 10.0,
            color: WHITE,
            font: ROBOTO,
        });

        let icon_name = if *is_dir { "music_dir" } else { "music_file" };
        icons.push(IconDraw {
            name: icon_name,
            x: button.x + PAD_2,
            y: button.y + PAD_2,
            width: 16.0,
            height: 16.0,
            tooltip: Tooltip {
                text: Some("Add Track"),
                x: button.x,
                y: button.y + 4.0,
            },
        });

        if button.is_hovered(mouse_state.x, mouse_state.y) {
            *cursor_icon = CursorIcon::Pointer;
            if mouse_state.left_clicked {
                if *is_dir {
                    if matches!(click_result, ClickResult::None) {
                        *click_result = ClickResult::FsToggleDir(path.clone());
                    }
                }
            }
        }

        *row += 1.0;

        if *is_dir && expanded_dirs.contains(path) {
            let line_x = button.x + 8.0;
            let line_top = y + 24.0;

            let mut child_icons = draw_fs_tree(
                path,
                depth + 1,
                row,
                base_y,
                expanded_dirs,
                mouse_state,
                screen_config,
                text_items,
                click_result,
                cursor_icon,
                fs_cache,
                tray_width,
                out,
            );
            icons.append(&mut child_icons);

            let line_bottom = base_y + *row * PAD_32;
            Rectangle {
                x: line_x,
                y: line_top,
                width: 1.0,
                height: line_bottom - line_top,
            }
            .draw(screen_config, DARK_GRAY_HOVER, NO_RADIUS, out);
        }
    }
    icons
}
