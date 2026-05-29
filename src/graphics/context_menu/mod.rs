use winit::window::CursorIcon;

use crate::{
    app::MouseState,
    graphics::{
        color::{Color, DARK_GRAY, DARK_GRAY_HOVER, DARK_GRAY_HOVER_HOVER, WHITE},
        font::{TextItem, ROBOTO},
        primitives::{ScreenConfig, PAD_2, PAD_32, PAD_4, PAD_64, PAD_8, RADIUS_4, RADIUS_8},
        widgets::Rectangle,
        ClickResult, Vertex,
    },
};

const CONTEXT_MENU_PADDING: f32 = 4.0;
const CONTEXT_MENU_ITEM_HEIGHT: f32 = 24.0;
const PATTERN_MENU_ITEM_COUNT: u32 = 6;
const CONTEXT_MENU_FONT_SIZE: f32 = 14.0;

pub enum ContextMenuKind {
    PatternContext(usize),
    TrackContext(usize, usize),
}

pub struct ContextMenu {
    pub kind: ContextMenuKind,
    pub x: f32,
    pub y: f32,
    pub width: f32,
}
fn menu_item_color(rect: &Rectangle, mx: f32, my: f32, held: bool) -> Color {
    if rect.is_hovered(mx, my) && !held {
        DARK_GRAY_HOVER_HOVER
    } else {
        DARK_GRAY_HOVER
    }
}

impl ContextMenu {
    pub fn draw(&self, screen_config: &ScreenConfig, mouse_state: &MouseState) -> (Vec<Vertex>, Vec<TextItem>, ClickResult, CursorIcon) {
        match &self.kind {
            ContextMenuKind::PatternContext(id) => self.draw_pattern_context(screen_config, mouse_state, *id),
            ContextMenuKind::TrackContext(pattern, track) => self.draw_track_context(screen_config, mouse_state, *pattern, *track),
        }
    }

    pub fn draw_background(&self) -> Rectangle {
        Rectangle {
            height: (CONTEXT_MENU_ITEM_HEIGHT + CONTEXT_MENU_PADDING) * PATTERN_MENU_ITEM_COUNT as f32 + CONTEXT_MENU_PADDING,
            width: self.width + PAD_8,
            x: self.x - PAD_64 - CONTEXT_MENU_PADDING,
            y: self.y + (CONTEXT_MENU_ITEM_HEIGHT + PAD_8) - CONTEXT_MENU_PADDING,
        }
    }

    pub fn draw_pattern_context_item_text(&self, index: usize) -> TextItem {
        let label = match index {
            0 => "Rename",
            1 => "Delete",
            2 => "Duplicate",
            _ => "N/A",
        };
        TextItem {
            text: label.to_string(),
            x: self.x - PAD_64 + PAD_4 + PAD_2,
            y: (self.y + (CONTEXT_MENU_ITEM_HEIGHT + PAD_4) * index as f32) + PAD_32,
            color: WHITE,
            font: ROBOTO,
            size: CONTEXT_MENU_FONT_SIZE,
        }
    }

    pub fn draw_track_context_item_text(&self, index: usize) -> TextItem {
        let label = match index {
            0 => "Rename",
            1 => "Piano Roll",
            4 => "Delete",
            _ => "N/A",
        };

        TextItem {
            text: label.to_string(),
            x: self.x - PAD_64 + PAD_4,
            y: (self.y + (CONTEXT_MENU_ITEM_HEIGHT + PAD_4) * index as f32) + PAD_32 + PAD_4,
            size: CONTEXT_MENU_FONT_SIZE,
            font: ROBOTO,
            color: WHITE,
        }
    }

    pub fn is_hovered(&self, mx: f32, my: f32) -> bool {
        let item_count = match &self.kind {
            ContextMenuKind::PatternContext(_) => 3,
            ContextMenuKind::TrackContext(_, _) => 5,
        };
        let rect = Rectangle {
            x: self.x - PAD_64,
            y: self.y + PAD_32,
            width: self.width,
            height: CONTEXT_MENU_ITEM_HEIGHT * item_count as f32 + PAD_8,
        };
        rect.is_hovered(mx, my)
    }

    fn draw_pattern_context(
        &self,
        screen_config: &ScreenConfig,
        mouse_state: &MouseState,
        id: usize,
    ) -> (Vec<Vertex>, Vec<TextItem>, ClickResult, CursorIcon) {
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut text_items: Vec<TextItem> = Vec::new();
        let mut cursor_icon = CursorIcon::Default;
        let mut click_result = ClickResult::None;

        // dark background
        let menu_background = self.draw_background();
        vertices.extend(menu_background.draw(screen_config, DARK_GRAY, RADIUS_8));

        // render each item - lighter background
        let item_x = self.x - PAD_64;
        for item in 0..PATTERN_MENU_ITEM_COUNT {
            // background for ContextMenu item
            let context_item_background = Rectangle {
                height: CONTEXT_MENU_ITEM_HEIGHT,
                width: self.width,
                x: item_x,
                y: (self.y + (CONTEXT_MENU_ITEM_HEIGHT + PAD_4) * item as f32) + PAD_32,
            };
            vertices.extend(context_item_background.draw(
                screen_config,
                menu_item_color(&context_item_background, mouse_state.x, mouse_state.y, mouse_state.left_clicked),
                RADIUS_4,
            ));

            text_items.push(self.draw_pattern_context_item_text(item as usize));

            if context_item_background.is_hovered(mouse_state.x, mouse_state.y) {
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
                        2 => {
                            // duplicate
                            click_result = ClickResult::DuplicatePattern(id);
                        }
                        _ => {
                            click_result = ClickResult::CloseContextMenu;
                        }
                    }
                }
            }
        }

        (vertices, text_items, click_result, cursor_icon)
    }

    fn draw_track_context(
        &self,
        screen_config: &ScreenConfig,
        mouse_state: &MouseState,
        pattern_id: usize,
        track_id: usize,
    ) -> (Vec<Vertex>, Vec<TextItem>, ClickResult, CursorIcon) {
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut text_items: Vec<TextItem> = Vec::new();
        let mut cursor_icon = CursorIcon::Default;
        let mut click_result = ClickResult::None;

        // dark background
        let menu_background = self.draw_background();
        vertices.extend(menu_background.draw(screen_config, DARK_GRAY, RADIUS_8));

        for item in 0..5 {
            let context_item_background = Rectangle {
                height: CONTEXT_MENU_ITEM_HEIGHT,
                width: self.width,
                x: self.x - PAD_64,
                y: (self.y + (CONTEXT_MENU_ITEM_HEIGHT + PAD_4) * item as f32) + PAD_32,
            };

            // background for ContextMenu item
            vertices.extend(context_item_background.draw(
                screen_config,
                menu_item_color(&context_item_background, mouse_state.x, mouse_state.y, mouse_state.left_clicked),
                RADIUS_4,
            ));

            // label for ContextMenu item
            text_items.push(self.draw_track_context_item_text(item as usize));

            if context_item_background.is_hovered(mouse_state.x, mouse_state.y) {
                cursor_icon = CursorIcon::Pointer;

                match item {
                    0 => {
                        // rename
                        {
                            if mouse_state.right_clicked || mouse_state.left_clicked {
                                click_result = ClickResult::CloseContextMenu;
                            }
                        }
                    }
                    1 => {
                        // piano roll
                        if mouse_state.right_clicked || mouse_state.left_clicked {
                            click_result = ClickResult::LoadPianoRoll(crate::app::PianoRollState {
                                pattern_id: (pattern_id),
                                track_id: (track_id as u32),
                            });
                        }
                    }

                    4 => {
                        // delete
                        if mouse_state.right_clicked || mouse_state.left_clicked {
                            click_result = ClickResult::DeleteTrack(track_id);
                        }
                    }
                    _ => {
                        if mouse_state.right_clicked || mouse_state.left_clicked {
                            click_result = ClickResult::CloseContextMenu;
                        }
                    }
                }
            }
        }

        (vertices, text_items, click_result, cursor_icon)
    }
}
