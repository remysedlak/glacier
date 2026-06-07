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
    screen_config: &ScreenConfig,
    mouse_state: &MouseState,
) -> (Vec<Vertex>, Vec<TextItem>, ClickResult, CursorIcon) {
    // setup
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut text_items: Vec<TextItem> = Vec::new();
    let mut click_result = ClickResult::None;
    let mut cursor_icon = CursorIcon::Default;

    // window background
    let window_background = window_background(window);
    vertices.extend(window_background.draw(
        screen_config,
        MINI_WINDOW_BACKGROUND,
        BOTTOM_RADIUS_16,
    ));

    // window titlebar
    let (titlebar_verts, titlebar_texts, result, cursor) =
        window_title_bar(window, "Mixer", screen_config, mouse_state);
    if !matches!(cursor, CursorIcon::Default) {
        cursor_icon = cursor;
    }
    click_result = click_result.or(result);
    vertices.extend(titlebar_verts);
    text_items.push(titlebar_texts);

    // master slider background
    let master_slider_x = window.x + SLIDER_BACKGROUND_OFFSET;
    let master_slider_y = window.y + SLIDER_BACKGROUND_OFFSET;
    let slider_background = Rectangle {
        x: master_slider_x - PAD_8,
        y: master_slider_y - PAD_8,
        width: MIXER_ITEM_WIDTH,
        height: window.height - PAD_32,
    };
    vertices.extend(slider_background.draw(screen_config, DARK_GRAY, NO_RADIUS));

    vertices.extend(draw_slider(
        master_volume,
        master_slider_x,
        slider_background.y + slider_background.height - 172.0,
        screen_config,
    ));
    text_items.push(TextItem {
        text: format!("{:.2}", master_volume),
        x: master_slider_x,
        y: master_slider_y + MIXER_TRACK_HEIGHT,
        size: BODY,
        font: MONOSPACED,
        color: LIGHT_GRAY,
    });

    // draw a track mixer for each track
    for track in tracks {
        let slider_x = (slider_background.x + PAD_16)
            + ((slider_background.width + PAD_4) * (track.data.id + 1) as f32);
        let slider_background = Rectangle {
            x: slider_x,
            y: slider_background.y,
            width: slider_background.width,
            height: slider_background.height,
        };
        vertices.extend(slider_background.draw(screen_config, DARK_GRAY, NO_RADIUS));

        // draw the actual slider knob and track
        vertices.extend(draw_slider(
            track.data.track_volume,
            slider_background.x + PAD_8,
            slider_background.y + slider_background.height - 172.0,
            screen_config,
        ));
        text_items.push(TextItem {
            text: format!("{:.2}", track.data.track_volume),
            x: slider_x + PAD_8,
            y: master_slider_y + MIXER_TRACK_HEIGHT,
            size: BODY,
            font: MONOSPACED,
            color: LIGHT_GRAY,
        });
    }

    (vertices, text_items, click_result, cursor_icon)
}
