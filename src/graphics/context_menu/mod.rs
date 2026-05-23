use winit::window::CursorIcon;

use crate::{
    app::MouseState,
    graphics::{
        color::{DARK_GRAY, DARK_GRAY_HOVER, WHITE},
        font::{TextItem, ROBOTO_FONT},
        primitives::{ScreenConfig, NO_RADIUS, PAD_2, PAD_24, PAD_32, PAD_4, PAD_64, PAD_8},
        widgets::Rectangle,
        ClickResult, Vertex,
    },
};

pub enum ContextMenuKind {
    PatternContext(usize),
    TrackContext(usize, usize),
}

pub struct ContextMenu {
    pub kind: ContextMenuKind,
    pub x: f32,
    pub y: f32,
    pub height: f32,
    pub width: f32,
}
fn menu_item_color(rect: &Rectangle, mx: f32, my: f32, held: bool) -> (f32, f32, f32) {
    if rect.is_hovered(mx, my) && !held {
        DARK_GRAY_HOVER
    } else {
        DARK_GRAY
    }
}

impl ContextMenu {
    pub fn draw(&self, screen_config: &ScreenConfig, mouse_state: &MouseState) -> (Vec<Vertex>, Vec<TextItem>, ClickResult, CursorIcon) {
        match &self.kind {
            ContextMenuKind::PatternContext(id) => self.draw_pattern_context(screen_config, mouse_state, *id),
            ContextMenuKind::TrackContext(pattern, track) => self.draw_track_context(screen_config, mouse_state, *pattern, *track),
        }
    }

    fn draw_pattern_context(
        &self,
        screen_config: &ScreenConfig,
        mouse_state: &MouseState,
        id: usize,
    ) -> (Vec<Vertex>, Vec<TextItem>, ClickResult, CursorIcon) {
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut texts: Vec<TextItem> = Vec::new();
        let mut cursor_icon = CursorIcon::Default;
        let mut click_result = ClickResult::None;

        let menu_background = Rectangle {
            height: 32.0 * 6.0,
            width: self.width + PAD_8,
            x: self.x - PAD_64 - PAD_4,
            y: self.y + (PAD_24 + PAD_8) - 4.0,
        };
        vertices.extend(menu_background.draw(screen_config, DARK_GRAY, NO_RADIUS));

        for item in 0..6 {
            let context_item = Rectangle {
                height: 24.0,
                width: self.width,
                x: self.x - PAD_64,
                y: (self.y + (PAD_24 + PAD_8) * item as f32) + PAD_32,
            };
            vertices.extend(context_item.draw(
                screen_config,
                menu_item_color(&context_item, mouse_state.x, mouse_state.y, mouse_state.left_clicked),
                NO_RADIUS,
            ));

            if context_item.is_hovered(mouse_state.x, mouse_state.y) {
                cursor_icon = CursorIcon::Pointer;
                if mouse_state.right_clicked || mouse_state.left_clicked {
                    match item {
                        0 => {
                            // rename
                            click_result = ClickResult::CloseContextMenu;
                        }
                        1 => {
                            // delete
                            click_result = ClickResult::DeletePattern(id);
                        }
                        _ => {
                            click_result = ClickResult::CloseContextMenu;
                        }
                    }
                }
            }
        }
        // delete text
        texts.push(TextItem {
            text: "Rename".to_string(),
            x: self.x - PAD_64 + PAD_4,
            y: (self.y + (PAD_24 + PAD_8) * 0 as f32) + PAD_32 + PAD_2,
            color: WHITE,
            font: ROBOTO_FONT,
            size: 14.0,
        });
        // delete text
        texts.push(TextItem {
            text: "Delete".to_string(),
            x: self.x - PAD_64 + PAD_4,
            y: (self.y + (PAD_24 + PAD_8) * 1 as f32) + PAD_32 + PAD_2,
            size: 14.0,
            font: ROBOTO_FONT,
            color: WHITE,
        });

        for line in 1..5 {
            let divider = Rectangle {
                height: 1.0,
                width: self.width + 4.0,
                x: (self.x + 2.0) - PAD_64 - 4.0,
                y: PAD_24 + (self.y + (PAD_24 + PAD_8) * line as f32) + PAD_32 + 4.0,
            };
            vertices.extend(divider.draw(screen_config, WHITE, NO_RADIUS));
        }
        (vertices, texts, click_result, cursor_icon)
    }

    fn draw_track_context(
        &self,
        screen_config: &ScreenConfig,
        mouse_state: &MouseState,
        pattern_id: usize,
        track_id: usize,
    ) -> (Vec<Vertex>, Vec<TextItem>, ClickResult, CursorIcon) {
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut texts: Vec<TextItem> = Vec::new();
        let mut cursor_icon = CursorIcon::Default;
        let mut click_result = ClickResult::None;
        for item in 0..5 {
            let context_item = Rectangle {
                height: 24.0,
                width: self.width,
                x: self.x - PAD_64,
                y: (self.y + PAD_24 * item as f32) + PAD_32,
            };
            vertices.extend(context_item.draw(
                screen_config,
                menu_item_color(&context_item, mouse_state.x, mouse_state.y, mouse_state.left_clicked),
                NO_RADIUS,
            ));
            if context_item.is_hovered(mouse_state.x, mouse_state.y) {
                cursor_icon = CursorIcon::Pointer;
                if mouse_state.right_clicked || mouse_state.left_clicked {
                    match item {
                        0 => {
                            // rename
                            click_result = ClickResult::CloseContextMenu;
                        }
                        1 => {
                            // piano roll
                            click_result = ClickResult::LoadPianoRoll(crate::app::PianoRollState {
                                pattern_id: (pattern_id),
                                instrument_id: (track_id as u32),
                            });
                        }
                        4 => {
                            // delete
                            click_result = ClickResult::DeleteTrack(track_id);
                        }
                        _ => {
                            click_result = ClickResult::CloseContextMenu;
                        }
                    }
                }
            }
        }

        for line in 0..4 {
            let divider = Rectangle {
                height: 1.0,
                width: self.width - 4.0,
                x: (self.x + 2.0) - PAD_64,
                y: PAD_24 + (self.y + PAD_24 * line as f32) + PAD_32,
            };
            vertices.extend(divider.draw(screen_config, WHITE, NO_RADIUS));
        }
        // delete text
        texts.push(TextItem {
            text: "Rename".to_string(),
            x: self.x - PAD_64 + PAD_4,
            y: (self.y + (PAD_24) * 0 as f32) + PAD_32 + PAD_2,
            size: 14.0,
            font: ROBOTO_FONT,
            color: WHITE,
        });
        texts.push(TextItem {
            text: "Delete".to_string(),
            x: self.x - PAD_64 + PAD_4,
            y: (self.y + (PAD_24) * 4 as f32) + PAD_32 + PAD_2,
            size: 14.0,
            font: ROBOTO_FONT,
            color: WHITE,
        });
        texts.push(TextItem {
            text: "Piano Roll".to_string(),
            x: self.x - PAD_64 + PAD_4,
            y: (self.y + (PAD_24) * 1 as f32) + PAD_32 + PAD_2,
            size: 14.0,
            font: ROBOTO_FONT,
            color: WHITE,
        });
        (vertices, texts, click_result, cursor_icon)
    }
}
