use crate::app::click::ClickResult;
use crate::graphics::primitives::{NO_RADIUS, PAD_16, PAD_2, PAD_32, PAD_4, RADIUS_4};
use crate::project::is_audio_file;
use crate::{
    app::MouseState,
    graphics::{
        color::*,
        font::{TextItem, ROBOTO},
        geometry::Rectangle,
        icons::IconDraw,
        ScreenConfig, Tooltip, Vertex,
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

    // base_y is already the exact y-coordinate of the file tree's real scissor top
    // (see draw.rs: divider_y = sh/2 + PAD_32 + PAD_16, and this function receives
    // base_y = divider_y_param + PAD_32 + PAD_16, which reconstructs that same value).
    // Do NOT subtract PAD_32/PAD_16 back out here — that was the bug: it produced a
    // cutoff line sitting PAD_32+PAD_16 px above the real scissor boundary.
    let visible_top = base_y;

    for (path, is_dir) in entries {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");

        let y = base_y + *row * PAD_32 - scroll_offset;

        // culling — "below the screen" means every remaining row is also below,
        // so it's safe to stop entirely. "above the visible top" only means THIS
        // row shouldn't draw its own rectangle/text/icon — its children may still
        // be visible further down, so we must NOT skip the recursive call for them.
        //
        // Two different thresholds are needed here:
        // - rect/text can rely on the real GPU scissor rect to correctly clip a row
        //   that's only PARTIALLY above the line (top cut, bottom visible) — so for
        //   them, "entirely above the line" is the right cull test.
        // - icons have NO scissor clipping at all (drawn in a separate unscissored
        //   pass), so a partially-overlapping icon would render in full and poke
        //   above the divider. Icons must be cut whenever ANY part of them could be
        //   above the line — i.e. the row's TOP, not its bottom, decides icon visibility.
        let above_visible = y + 24.0 < visible_top; // whole row above the line — skip rect/text
        let icon_visible = y >= visible_top; // row's top itself below the line — safe for icon
        if y > screen_config.height as f32 {
            return icons;
        }

        if !above_visible {
            let button = Rectangle {
                height: 24.0,
                width: tray_width - PAD_4 * 2.0 - indent,
                x: PAD_4 + indent,
                y,
            };
            let color = if button.is_hovered(mouse_state.x, mouse_state.y) {
                DARK_GRAY
            } else {
                SURFACE
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
            if icon_visible {
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
            }

            if button.is_hovered(mouse_state.x, mouse_state.y) {
                // show pointer for directories and cursor for files
                *cursor_icon = if *is_dir {
                    CursorIcon::Pointer
                } else {
                    CursorIcon::Default
                };
                if !*is_dir {
                    if is_audio_file(path) {
                        if mouse_state.left_clicked && matches!(click_result, ClickResult::None) {
                            *click_result = ClickResult::FsPreviewSample(path.clone());
                        }
                        if mouse_state.left_click_held && matches!(click_result, ClickResult::None)
                        {
                            *click_result = ClickResult::FsStartDragFile(path.clone());
                        }
                    }
                    // non-audio files do nothing
                } else if mouse_state.left_clicked && matches!(click_result, ClickResult::None) {
                    *click_result = ClickResult::FsToggleDir(path.clone());
                }
            }
        }

        *row += 1.0;

        if *is_dir && expanded_dirs.contains(path) {
            let line_x = PAD_4 + indent + 8.0;
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

            // ui visual nesting of items within a directory — only draw this line
            // if the parent row itself was visible; if it wasn't, its top anchor
            // point is off-screen and drawing the line would be meaningless
            if !above_visible {
                let line_bottom = base_y + *row * PAD_32 - scroll_offset;
                Rectangle {
                    x: line_x,
                    y: line_top,
                    width: 1.0,
                    height: line_bottom - line_top - 2.0,
                }
                .draw(screen_config, DARK_GRAY, NO_RADIUS, out);
            }
        }
    }
    icons
}
