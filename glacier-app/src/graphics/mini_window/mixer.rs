use winit::window::CursorIcon;

use crate::{
    app::MouseState,
    graphics::{
        color::*,
        font::{BODY, MONOSPACED},
        mini_window::MiniWindow,
        primitives::{ScreenConfig, BOTTOM_RADIUS_16, NO_RADIUS, PAD_16, PAD_32, PAD_4, PAD_8},
        widgets::{
            draw_slider, window_background, window_title_bar, Rectangle, MIXER_TRACK_HEIGHT,
        },
        ClickResult, TextItem, Vertex,
    },
    project::Track,
};

pub const SLIDER_BACKGROUND_OFFSET: f32 = PAD_16;
pub const MIXER_ITEM_WIDTH: f32 = 50.0;

pub fn draw(
    window: &MiniWindow,
    tracks: &[Track],
    master_volume: f32,
    master_rms_l: f32,
    master_rms_r: f32,
    master_peak: f32,
    screen_config: &ScreenConfig,
    mouse_state: &MouseState,
) -> (Vec<Vertex>, Vec<TextItem>, ClickResult, CursorIcon) {
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut text_items: Vec<TextItem> = Vec::new();
    let mut click_result = ClickResult::None;
    let mut cursor_icon = CursorIcon::Default;

    let window_background = window_background(window);
    vertices.extend(window_background.draw(
        screen_config,
        MINI_WINDOW_BACKGROUND,
        BOTTOM_RADIUS_16,
    ));

    let (titlebar_verts, titlebar_texts, result, cursor) =
        window_title_bar(window, "Mixer", screen_config, mouse_state);
    if !matches!(cursor, CursorIcon::Default) {
        cursor_icon = cursor;
    }
    click_result = click_result.or(result);
    vertices.extend(titlebar_verts);
    text_items.push(titlebar_texts);

    let master_slider_x = window.x + SLIDER_BACKGROUND_OFFSET;
    let master_slider_y = window.y + SLIDER_BACKGROUND_OFFSET;

    let col_height = window.height - PAD_32;
    let meter_area_height = col_height * 0.5;
    let slider_area_height = col_height * 0.5;

    // helper closure to draw one channel strip
    let mut draw_channel = |vertices: &mut Vec<Vertex>,
                            text_items: &mut Vec<TextItem>,
                            col_x: f32,
                            volume: f32,
                            rms_l: f32,
                            rms_r: f32,
                            peak: f32| {
        let bg = Rectangle {
            x: col_x - PAD_8,
            y: master_slider_y - PAD_8,
            width: MIXER_ITEM_WIDTH,
            height: col_height,
        };
        vertices.extend(bg.draw(screen_config, DARK_GRAY, NO_RADIUS));

        // meter top half
        let meter_bg = Rectangle {
            x: col_x,
            y: bg.y + PAD_8,
            width: MIXER_ITEM_WIDTH - PAD_16,
            height: meter_area_height - PAD_32,
        };
        vertices.extend(meter_bg.draw(screen_config, BLACK, NO_RADIUS));

        let bar_width = (meter_bg.width - 2.0) * 0.5; // 2px gap between

        // left bar
        let fill_l = (meter_bg.height * rms_l.clamp(0.0, 1.0)).min(meter_bg.height);
        let bar_l = Rectangle {
            x: meter_bg.x,
            y: meter_bg.y + meter_bg.height - fill_l,
            width: bar_width,
            height: fill_l,
        };
        vertices.extend(bar_l.draw(screen_config, GREEN, NO_RADIUS));

        // right bar
        let fill_r = (meter_bg.height * rms_r.clamp(0.0, 1.0)).min(meter_bg.height);
        let bar_r = Rectangle {
            x: meter_bg.x + bar_width + 2.0,
            y: meter_bg.y + meter_bg.height - fill_r,
            width: bar_width,
            height: fill_r,
        };
        vertices.extend(bar_r.draw(screen_config, GREEN, NO_RADIUS));

        // peak line spans full width
        let peak_y = (meter_bg.y + meter_bg.height - (meter_bg.height * peak.clamp(0.0, 1.0)))
            .max(meter_bg.y);
        let peak_line = Rectangle {
            x: meter_bg.x,
            y: peak_y,
            width: meter_bg.width,
            height: 2.0,
        };
        vertices.extend(peak_line.draw(screen_config, ORANGE, NO_RADIUS));

        // slider bottom half
        let slider_y = bg.y + meter_area_height + slider_area_height - 172.0;
        vertices.extend(draw_slider(volume, col_x, slider_y, screen_config));

        text_items.push(TextItem {
            text: format!("{:.2}", volume),
            x: col_x,
            y: master_slider_y + MIXER_TRACK_HEIGHT,
            size: BODY,
            font: MONOSPACED,
            color: LIGHT_GRAY,
        });
    };

    draw_channel(
        &mut vertices,
        &mut text_items,
        master_slider_x,
        master_volume,
        master_rms_l,
        master_rms_r,
        master_peak,
    );

    for track in tracks {
        let col_x = (master_slider_x - PAD_8 + PAD_16)
            + ((MIXER_ITEM_WIDTH + PAD_4) * (track.data.id + 1) as f32);
        draw_channel(
            &mut vertices,
            &mut text_items,
            col_x,
            track.data.track_volume,
            track.rms_l,
            track.rms_r,
            track.peak_hold,
        );
    }

    (vertices, text_items, click_result, cursor_icon)
}
