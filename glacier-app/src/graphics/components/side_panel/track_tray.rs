use crate::graphics::color::DARK_GRAY_HOVER;
use crate::graphics::icons::IconDraw;
use crate::graphics::Tooltip;
use crate::graphics::ICON_SIZE;
use crate::project::Track;
use crate::{
    app::MouseState,
    graphics::{
        color::{BLACK, LIGHT_GRAY, LIGHT_GRAY_HOVER, PEBBLE, WHITE},
        components::side_panel::{PATTERN_TRAY_HEADER_MARGIN, PATTERN_TRAY_ITEM_GAP, TRAY_WIDTH},
        font::{truncate_text, TextItem, ROBOTO},
        primitives::*,
        side_panel::{draw_title, PATTERN_TRAY_ITEM_HEIGHT},
        widgets::{Rectangle, TOOLBAR_Y},
        ClickResult,
    },
};
use winit::window::CursorIcon;

pub fn draw(
    mouse_state: &MouseState,
    screen_config: &ScreenConfig,
    tracks: &[Track],
    user_fs_location: &std::path::Path,
    expanded_dirs: &std::collections::HashSet<std::path::PathBuf>,
    out: &mut Vec<Vertex>,
) -> (Vec<TextItem>, Vec<IconDraw>, ClickResult, CursorIcon) {
    // setup

    let mut text_items: Vec<TextItem> = Vec::new();
    let mut cursor_icon: CursorIcon = CursorIcon::Default;
    let mut click_result: ClickResult = ClickResult::None;

    let track_tray = Rectangle {
        x: 0.0,
        y: TOOLBAR_Y,
        width: TRAY_WIDTH,
        height: screen_config.height as f32 - TOOLBAR_Y,
    };
    track_tray.draw(screen_config, PEBBLE, NO_RADIUS, out);

    text_items.push(draw_title("Tracks", (track_tray.x, track_tray.y)));

    for (i, track) in tracks.iter().enumerate() {
        let button_x = track_tray.x + PAD_16;
        let button_y = PATTERN_TRAY_HEADER_MARGIN + (PATTERN_TRAY_ITEM_GAP * i as f32) + PAD_32;
        let track_button = Rectangle {
            x: button_x,
            y: button_y,
            width: track_tray.width - PAD_32,
            height: PATTERN_TRAY_ITEM_HEIGHT,
        };

        let track_button_color = if track_button.is_hovered(mouse_state.x, mouse_state.y) {
            LIGHT_GRAY_HOVER
        } else {
            LIGHT_GRAY
        };

        track_button.draw(screen_config, track_button_color, RADIUS_4, out);
        if track_button.is_hovered(mouse_state.x, mouse_state.y) {
            cursor_icon = CursorIcon::Pointer;
            if mouse_state.left_double_clicked {
                click_result = ClickResult::ToggleTrackWindow(i);
            }
        }

        text_items.push(TextItem {
            text: truncate_text(&track.data.name, 9),
            x: track_button.x + PAD_4,
            y: PATTERN_TRAY_HEADER_MARGIN + (PATTERN_TRAY_ITEM_GAP * i as f32) + PAD_32 + PAD_2,
            size: 16.0,
            color: BLACK,
            font: ROBOTO,
        });
    }
    // section for os files
    //
    //

    let divider = Rectangle {
        x: 0.0,
        y: (screen_config.height / 2) as f32,
        width: TRAY_WIDTH,
        height: 1.0,
    };
    divider.draw(screen_config, WHITE, RADIUS_4, out);
    text_items.push(draw_title("fs", (divider.x, divider.y)));

    let mut row: f32 = 0.0;
    let icons = draw_fs_tree(
        user_fs_location,
        0,
        &mut row,
        divider.y + PAD_32,
        expanded_dirs,
        mouse_state,
        screen_config,
        &mut text_items,
        &mut click_result,
        &mut cursor_icon,
        out,
    );

    (text_items, icons, click_result, cursor_icon)
}

fn draw_fs_tree(
    dir: &std::path::Path,
    depth: usize,
    row: &mut f32,
    base_y: f32,
    expanded_dirs: &std::collections::HashSet<std::path::PathBuf>,
    mouse_state: &MouseState,
    screen_config: &ScreenConfig,
    text_items: &mut Vec<TextItem>,
    click_result: &mut ClickResult,
    cursor_icon: &mut CursorIcon,
    out: &mut Vec<Vertex>,
) -> Vec<IconDraw> {
    let mut icons: Vec<IconDraw> = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let indent = depth as f32 * PAD_16;

    for entry in entries.flatten() {
        let path = entry.path();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");

        let y = base_y + *row * PAD_32;
        let button = Rectangle {
            height: 24.0,
            width: TRAY_WIDTH - PAD_4 * 2.0 - indent,
            x: PAD_4 + indent,
            y,
        };
        button.draw(screen_config, DARK_GRAY_HOVER, RADIUS_4, out);

        // item text
        text_items.push(TextItem {
            text: truncate_text(name, 25),
            x: button.x + PAD_4 + 16.0,
            y: button.y + PAD_4,
            size: 10.0,
            color: WHITE,
            font: ROBOTO,
        });
        // item icon
        let icon_name = if is_dir { "music_dir" } else { "music_file" };
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
                if is_dir {
                    if matches!(click_result, ClickResult::None) {
                        *click_result = ClickResult::FsToggleDir(path.clone());
                    }
                } else {
                    // eventually: load sample
                }
            }
        }

        *row += 1.0;

        if is_dir && expanded_dirs.contains(&path) {
            let mut child_icons = draw_fs_tree(
                &path,
                depth + 1,
                row,
                base_y,
                expanded_dirs,
                mouse_state,
                screen_config,
                text_items,
                click_result,
                cursor_icon,
                out,
            );
            icons.append(&mut child_icons);
        }
    }
    icons
}
