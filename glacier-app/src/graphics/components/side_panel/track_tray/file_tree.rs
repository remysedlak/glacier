use crate::{
    app::MouseState,
    graphics::{
        color::*,
        font::{TextItem, ROBOTO},
        icons::IconDraw,
        primitives::*,
        widgets::{Rectangle, TOOLBAR_Y},
        ClickResult, Tooltip,
    },
};
use std::collections::HashMap;
use std::path::PathBuf;
use winit::window::CursorIcon;

pub fn draw(
    mouse_state: &MouseState,
    screen_config: &ScreenConfig,
    user_fs_location: &std::path::Path,
    expanded_dirs: &std::collections::HashSet<PathBuf>,
    fs_cache: &HashMap<PathBuf, Vec<(PathBuf, bool)>>,
    scroll_offset: f32,
    tray_width: f32,
    out: &mut Vec<Vertex>,
    divider_y: f32,
) -> (Vec<IconDraw>, Vec<TextItem>, ClickResult, CursorIcon) {
    let mut text_items: Vec<TextItem> = Vec::new();
    let mut cursor: CursorIcon = CursorIcon::Default;
    let mut click = ClickResult::None;

    let mut row: f32 = 0.0;
    (
        draw_fs_tree(
            user_fs_location,
            0,
            &mut row,
            divider_y + PAD_32 + PAD_16,
            expanded_dirs,
            mouse_state,
            screen_config,
            &mut text_items,
            &mut click,
            &mut cursor,
            fs_cache,
            scroll_offset,
            tray_width,
            out,
        ),
        text_items,
        click,
        cursor,
    )
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
    scroll_offset: f32,
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

        let y = base_y + *row * PAD_32 - scroll_offset;

        // culling
        //
        if y + 24.0 < TOOLBAR_Y {
            *row += 1.0;
            continue;
        }
        if y > screen_config.height as f32 {
            return icons;
        }
        //
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
            text: name.to_string(),
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

            // show pointer for directories and cursor for files
            *cursor_icon = if *is_dir {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            };
            if !*is_dir {
                if mouse_state.left_clicked && matches!(click_result, ClickResult::None) {
                    *click_result = ClickResult::FsPreviewSample(path.clone());
                }
                if mouse_state.left_click_held && matches!(click_result, ClickResult::None) {
                    *click_result = ClickResult::FsStartDragFile(path.clone());
                }
            } else if mouse_state.left_clicked && matches!(click_result, ClickResult::None) {
                *click_result = ClickResult::FsToggleDir(path.clone());
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
                scroll_offset,
                tray_width,
                out,
            );
            icons.append(&mut child_icons);

            // ui visual nesting of items within a directory
            let line_bottom = base_y + *row * PAD_32 - scroll_offset;
            Rectangle {
                x: line_x,
                y: line_top,
                width: 1.0,
                height: line_bottom - line_top - 2.0,
            }
            .draw(screen_config, DARK_GRAY_HOVER, NO_RADIUS, out);
        }
    }
    icons
}
