use crate::app::click::ClickResult;
use crate::graphics::components::toolbar::TOOLBAR_Y;
use crate::graphics::geometry::Rectangle;
use crate::graphics::mini_window::InteractionResult;
use crate::project::{Track, TrackID};
use crate::{
    app::MouseState,
    graphics::{
        color::*,
        components::side_panel::{PATTERN_TRAY_HEADER_MARGIN, PATTERN_TRAY_ITEM_GAP},
        font::{truncate_text, Font::Roboto, TextItem},
        primitives::*,
        side_panel::{draw_title, PATTERN_TRAY_ITEM_HEIGHT},
    },
};
use winit::window::CursorIcon;

pub mod file_tree;

pub fn draw(
    mouse_state: &MouseState,
    screen_config: &ScreenConfig,
    resizing: bool,
    tracks: &[Track],
    tray_width: f32,
    out: &mut Vec<Vertex>,
    selected_track_id: Option<TrackID>,
) -> (Vec<TextItem>, InteractionResult) {
    let mut text_items: Vec<TextItem> = Vec::new();
    let mut interaction = InteractionResult::default();

    let track_tray = Rectangle {
        x: 0.0,
        y: TOOLBAR_Y,
        width: tray_width,
        height: screen_config.height as f32 - TOOLBAR_Y,
    };
    track_tray.draw(screen_config, SURFACE, NO_RADIUS, out);

    if track_tray.is_hovered_right_edge(mouse_state.x, mouse_state.y) || resizing {
        interaction.cursor = CursorIcon::ColResize;
    }

    text_items.push(draw_title("Tracks", (track_tray.x, track_tray.y)));

    for (i, track) in tracks.iter().enumerate() {
        let button_y = PATTERN_TRAY_HEADER_MARGIN + (PATTERN_TRAY_ITEM_GAP * i as f32) + PAD_32;
        let track_button = Rectangle {
            y: button_y,
            height: PATTERN_TRAY_ITEM_HEIGHT,
            ..track_tray.inset_x(PAD_4)
        };

        let track_button_color = if track_button.is_hovered(mouse_state.x, mouse_state.y) {
            SURFACE_HOVER
        } else {
            SURFACE
        };

        track_button.draw(screen_config, track_button_color, RADIUS_4, out);
        if track_button.is_hovered(mouse_state.x, mouse_state.y) {
            interaction.cursor = CursorIcon::Pointer;
            if mouse_state.left_double_clicked {
                interaction.click = ClickResult::ToggleTrackWindow(TrackID(i as u32));
            } else if mouse_state.left_clicked {
                interaction.click = ClickResult::SelectTrackTray(track.data.id);
            }
        }
        let is_selected = selected_track_id == Some(track.data.id);
        if is_selected {
            let signal = Rectangle {
                x: track_button.x,
                y: button_y,
                width: 4.0,
                height: PATTERN_TRAY_ITEM_HEIGHT,
            };
            signal.draw(screen_config, ORANGE, RADIUS_4, out);
        }

        let text_pos = track_button.offset(PAD_8, PAD_2);
        text_items.push(TextItem {
            text: truncate_text(&track.data.name, 18),
            x: text_pos.x,
            y: text_pos.y,
            size: 10.0,
            color: WHITE,
            font: Roboto,
        });
    }

    (text_items, interaction)
}
